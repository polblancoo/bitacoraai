//! Explorador de archivos de solo lectura de la carpeta de trabajo de un proyecto: para quien usa la web desde otra
//! máquina y no puede abrir esa carpeta. No sale de la carpeta (se resuelven los enlaces simbólicos), no muestra
//! archivos ocultos ni nada que parezca una clave, y solo entrega texto de tamaño razonable.
use crate::{auth::AuthUser, error::{AppError, Result}, routes, AppState};
use axum::{extract::{Path, Query, State}, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path as FsPath, PathBuf};
use tm_core::Id;

const MAX_FILE: u64 = 512 * 1024;
const MAX_ENTRIES: usize = 500;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/projects/{id}/files", get(list)).route("/api/projects/{id}/file", get(read))
}

#[derive(Deserialize)]
struct Q {
    #[serde(default)]
    path: String,
}

/// Nombres que nunca se listan ni se leen: ocultos, claves, bases de datos y carpetas de compilación o dependencias.
fn hidden(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with('.')
        || matches!(n.as_str(), "target" | "node_modules" | "id_rsa" | "id_ed25519" | "credentials" | "secrets")
        || [".pem", ".key", ".p12", ".pfx", ".db", ".sqlite", ".sqlite3", ".env", ".keystore", ".jks"].iter().any(|e| n.ends_with(e))
        || n.contains("secret")
        || n.contains("password")
}

/// Resuelve `rel` dentro de la carpeta de trabajo del proyecto; rechaza todo lo que salga de ella o esté oculto.
async fn resolve(st: &AppState, user: &AuthUser, project: Id, rel: &str) -> Result<(PathBuf, PathBuf)> {
    user.human_only()?;
    let team = routes::team_of_project(st, project).await?;
    routes::ensure_writer(st, team, user.id).await?;
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    if dir.trim().is_empty() {
        return Err(AppError::bad("este proyecto no tiene carpeta de trabajo"));
    }
    let base = std::fs::canonicalize(&dir).map_err(|_| AppError::bad("la carpeta no existe en la máquina del servidor"))?;
    let mut target = base.clone();
    for part in rel.split('/').filter(|p| !p.is_empty()) {
        if part == ".." || part == "." || part.contains('\0') || hidden(part) {
            return Err(AppError::bad("ruta no permitida"));
        }
        target.push(part);
    }
    let target = std::fs::canonicalize(&target).map_err(|_| AppError::bad("no existe"))?;
    if !target.starts_with(&base) || target.strip_prefix(&base).is_ok_and(|r| r.components().any(|c| hidden(&c.as_os_str().to_string_lossy()))) {
        return Err(AppError::bad("ruta no permitida"));
    }
    Ok((base, target))
}

async fn list(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Query(q): Query<Q>) -> Result<Json<Value>> {
    let (base, dir) = resolve(&st, &user, project, &q.path).await?;
    let rd = std::fs::read_dir(&dir).map_err(|_| AppError::bad("no es una carpeta"))?;
    let mut items: Vec<(bool, String, u64)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if hidden(&name) {
                return None;
            }
            // se sigue el enlace, pero solo se muestra si queda dentro de la carpeta
            let real = std::fs::canonicalize(e.path()).ok().filter(|p| p.starts_with(&base))?;
            let md = std::fs::metadata(&real).ok()?;
            Some((md.is_dir(), name, md.len()))
        })
        .collect();
    items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    let truncated = items.len() > MAX_ENTRIES;
    items.truncate(MAX_ENTRIES);
    let rel = dir.strip_prefix(&base).unwrap_or(FsPath::new("")).to_string_lossy().into_owned();
    Ok(Json(json!({
        "path": rel,
        "truncated": truncated,
        "entries": items.iter().map(|(d, n, s)| json!({ "name": n, "dir": d, "size": s })).collect::<Vec<_>>(),
    })))
}

async fn read(State(st): State<AppState>, user: AuthUser, Path(project): Path<Id>, Query(q): Query<Q>) -> Result<Json<Value>> {
    let (_, file) = resolve(&st, &user, project, &q.path).await?;
    let md = std::fs::metadata(&file).map_err(|_| AppError::bad("no existe"))?;
    if !md.is_file() {
        return Err(AppError::bad("no es un archivo"));
    }
    if md.len() > MAX_FILE {
        return Err(AppError::bad("el archivo es muy grande para verlo acá (más de 512 KB)"));
    }
    let bytes = std::fs::read(&file).map_err(|_| AppError::bad("no se pudo leer"))?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::bad("no es un archivo de texto"))?;
    crate::extras::log(&st, project, user.id, format!("abrió el archivo «{}» desde el explorador", q.path)).await;
        Ok(Json(json!({ "path": q.path, "size": md.len(), "text": text })))
}

#[cfg(test)]
mod tests {
    use super::hidden;

    #[test]
    fn oculta_claves_y_archivos_ocultos() {
        for n in [".env", ".git", "id_rsa", "clave.pem", "app.db", "mi-secret.txt", "target", "node_modules", "token.KEY"] {
            assert!(hidden(n), "{n}");
        }
        for n in ["main.rs", "README.md", "src", "Cargo.toml", "style.css"] {
            assert!(!hidden(n), "{n}");
        }
    }
}
