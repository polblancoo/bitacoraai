//! Aprobación por riesgo: en vez de listas de comandos sueltas, el CTO decide por tipo de acción
//! (borrar, publicar, desplegar, instalar, administrar el sistema) si las IAs la hacen libremente, necesitan
//! que una persona la apruebe (`/permitir`) o está prohibida. «Pedir» se apoya en lo que ya existe:
//! la regla queda como prohibida hasta que alguien aprueba un comando de esa categoría para la tarea.
use crate::{auth::AuthUser, error::{AppError, Result}, license, sysops, AppState};
use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use tm_core::Id;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/admin/risk", get(read).put(save))
}

/// (clave, nombre, qué incluye, reglas de Claude Code)
pub const CATS: &[(&str, &str, &str, &[&str])] = &[
    ("borrar", "🗑 Borrar", "rm, rmdir, git clean, git reset --hard, borrar ramas", &["Bash(rm:*)", "Bash(rmdir:*)", "Bash(git clean:*)", "Bash(git reset --hard:*)", "Bash(git branch -D:*)"]),
    ("publicar", "📤 Publicar", "git push, publicar paquetes, releases y merge de PRs", &["Bash(git push:*)", "Bash(npm publish:*)", "Bash(cargo publish:*)", "Bash(twine upload:*)", "Bash(gh release:*)", "Bash(gh pr merge:*)"]),
    ("desplegar", "🚀 Desplegar", "kubectl, terraform, helm, ansible, docker push, ssh, scp, rsync", &["Bash(kubectl:*)", "Bash(terraform:*)", "Bash(helm:*)", "Bash(ansible:*)", "Bash(docker push:*)", "Bash(ssh:*)", "Bash(scp:*)", "Bash(rsync:*)"]),
    ("instalar", "📦 Instalar", "npm/pip/cargo install, apt, brew, curl y wget", &["Bash(npm install:*)", "Bash(pip install:*)", "Bash(pip3 install:*)", "Bash(cargo install:*)", "Bash(apt:*)", "Bash(apt-get:*)", "Bash(brew install:*)", "Bash(curl:*)", "Bash(wget:*)"]),
    ("sistema", "🛠 Administrar el sistema", "sudo, chmod/chown, systemctl, reiniciar", &["Bash(sudo:*)", "Bash(chmod:*)", "Bash(chown:*)", "Bash(systemctl:*)", "Bash(reboot:*)", "Bash(shutdown:*)"]),
];

pub const MODES: [&str; 3] = ["libre", "pedir", "prohibido"];

async fn modes(st: &AppState) -> HashMap<&'static str, String> {
    let mut m = HashMap::new();
    for (k, ..) in CATS {
        let v = license::meta_get(&st.db, &format!("risk_{k}")).await.unwrap_or_default();
        m.insert(*k, if MODES.contains(&v.as_str()) { v } else { "libre".to_string() });
    }
    m
}

/// El comando de una regla o de un permiso, sin `Bash(` ni `:*)`: «Bash(rm:*)» y «rm -rf x» → «rm».
fn base(rule: &str) -> String {
    let r = rule.trim();
    let r = r.strip_prefix("Bash(").map(|x| x.trim_end_matches(')').trim_end_matches(":*")).unwrap_or(r);
    r.trim().to_string()
}

/// ¿Este permiso aprobado cubre la regla de la categoría? («rm -rf build» cubre «Bash(rm:*)»; «rmdir» no.)
fn covers(approved: &str, rule: &str) -> bool {
    let (a, r) = (base(approved), base(rule));
    !r.is_empty() && (a == r || a.starts_with(&format!("{r} ")))
}

/// Reglas prohibidas que la política de riesgo suma para una IA. En «pedir», una categoría se libera cuando hay un permiso
/// aprobado que la cubre: uno «de una vez» sin consumir de esta tarea, o uno permanente de la IA o de todo el servidor.
pub async fn deny_rules(st: &AppState, bot: Id, task: Option<Id>) -> Vec<String> {
    let m = modes(st).await;
    let mut approved: Vec<String> = Vec::new();
    if let Some(t) = task {
        approved.extend(sqlx::query_scalar::<_, String>("SELECT pattern FROM task_grants WHERE task_id = ? AND used = 0").bind(t).fetch_all(&st.db).await.unwrap_or_default());
    }
    let own: String = sqlx::query_scalar("SELECT allow FROM users WHERE id = ?").bind(bot).fetch_one(&st.db).await.unwrap_or_default();
    let general = license::meta_get(&st.db, "cmd_allow").await.unwrap_or_default();
    approved.extend(own.lines().chain(general.lines()).map(str::to_string));
    let mut out = Vec::new();
    for (k, _, _, rules) in CATS {
        match m[k].as_str() {
            "prohibido" => out.extend(rules.iter().map(|r| r.to_string())),
            "pedir" => out.extend(rules.iter().filter(|r| !approved.iter().any(|a| covers(a, r))).map(|r| r.to_string())),
            _ => {}
        }
    }
    out
}

async fn read(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    sysops::need(&st, &user, false).await?;
    let m = modes(&st).await;
    Ok(Json(json!({ "cats": CATS.iter().map(|(k, name, what, rules)| json!({ "key": k, "name": name, "what": what, "rules": rules, "mode": m[k] })).collect::<Vec<_>>() })))
}

#[derive(Deserialize)]
struct Save {
    modes: HashMap<String, String>,
}

async fn save(State(st): State<AppState>, user: AuthUser, Json(b): Json<Save>) -> Result<Json<Value>> {
    sysops::need(&st, &user, false).await?;
    for (k, v) in &b.modes {
        if !CATS.iter().any(|(c, ..)| c == k) || !MODES.contains(&v.as_str()) {
            return Err(AppError::bad("categoría o modo inválido"));
        }
    }
    let before = modes(&st).await;
    let mut changed = Vec::new();
    for (k, v) in &b.modes {
        if before[k.as_str()] != *v {
            license::meta_set(&st.db, &format!("risk_{k}"), v).await?;
            changed.push(format!("{k}: {} → {v}", before[k.as_str()]));
        }
    }
    if !changed.is_empty() {
        // la política es del servidor: queda anotada en la auditoría de cada equipo de quien la cambió
        let teams: Vec<Id> = sqlx::query("SELECT team_id FROM team_members WHERE user_id = ?").bind(user.id).fetch_all(&st.db).await?.iter().map(|r| r.get(0)).collect();
        for t in teams {
            crate::audit::record(&st, t, None, None, user.id, "politica_riesgo", &changed.join("; ")).await;
        }
    }
    read(State(st), user).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permisos_que_cubren() {
        assert!(covers("rm -rf build", "Bash(rm:*)"));
        assert!(covers("Bash(rm:*)", "Bash(rm:*)"));
        assert!(!covers("rmdir x", "Bash(rm:*)"));
        assert!(covers("git push origin main", "Bash(git push:*)"));
        assert!(!covers("git status", "Bash(git push:*)"));
        assert!(!covers("curl", "Bash(git push:*)"));
    }
}
