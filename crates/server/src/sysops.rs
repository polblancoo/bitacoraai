//! Operación del servidor: rol de servidor (usuario / CTO / administrador) y la vista «Procesos» con el uso de recursos de la app.
//! Es independiente de los roles de cada equipo: gobierna qué ve cada persona de la instalación completa.
use crate::{auth::AuthUser, error::{AppError, Result}, AppState};
use axum::{extract::{Path, State}, routing::{get, put}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::time::Duration;
use tm_core::Id;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/procesos", get(procesos))
        .route("/api/admin/users", get(users))
        .route("/api/admin/users/{id}/role", put(set_role))
}

/// Rol de servidor de una persona (`user` si no es válida o es una IA).
pub async fn server_role(st: &AppState, user: &AuthUser) -> String {
    if user.bot {
        return "user".into();
    }
    sqlx::query_scalar::<_, String>("SELECT server_role FROM users WHERE id = ?").bind(user.id).fetch_optional(&st.db).await.ok().flatten().unwrap_or_else(|| "user".into())
}

async fn need(st: &AppState, user: &AuthUser, admin_only: bool) -> Result<()> {
    user.human_only()?;
    let r = server_role(st, user).await;
    if r == "admin" || (!admin_only && r == "cto") { Ok(()) } else { Err(AppError::forbidden()) }
}

async fn users(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, true).await?;
    let rows = sqlx::query("SELECT id, username, server_role FROM users WHERE is_bot = 0 AND id NOT IN (SELECT user_id FROM api_tokens) ORDER BY username").fetch_all(&st.db).await?;
    Ok(Json(json!(rows.iter().map(|r| json!({ "id": r.get::<i64, _>(0), "username": r.get::<String, _>(1), "role": r.get::<String, _>(2) })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct RoleIn {
    role: String,
}

async fn set_role(State(st): State<AppState>, user: AuthUser, Path(id): Path<Id>, Json(b): Json<RoleIn>) -> Result<Json<Value>> {
    need(&st, &user, true).await?;
    if !matches!(b.role.as_str(), "user" | "cto" | "admin") {
        return Err(AppError::bad("rol no válido"));
    }
    let cur: Option<String> = sqlx::query_scalar("SELECT server_role FROM users WHERE id = ? AND is_bot = 0 AND id NOT IN (SELECT user_id FROM api_tokens)").bind(id).fetch_optional(&st.db).await?;
    let cur = cur.ok_or_else(|| AppError::bad("persona no encontrada"))?;
    if cur == "admin" && b.role != "admin" {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE server_role = 'admin'").fetch_one(&st.db).await?;
        if n <= 1 {
            return Err(AppError::bad("tiene que quedar al menos una persona administradora"));
        }
    }
    sqlx::query("UPDATE users SET server_role = ? WHERE id = ?").bind(&b.role).bind(id).execute(&st.db).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn read(p: &str) -> String {
    tokio::fs::read_to_string(p).await.unwrap_or_default()
}

fn kb_of(s: &str, key: &str) -> u64 {
    s.lines().find(|l| l.starts_with(key)).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// (pid, nombre, tiempo de CPU en ticks, RSS en KB) de los procesos de esta app.
async fn snapshot() -> Vec<(u32, String, u64, u64)> {
    let mut out = vec![];
    let Ok(mut d) = tokio::fs::read_dir("/proc").await else { return out };
    while let Ok(Some(e)) = d.next_entry().await {
        let Some(pid) = e.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
        let stat = read(&format!("/proc/{pid}/stat")).await;
        let (Some(a), Some(b)) = (stat.find('('), stat.rfind(')')) else { continue };
        let name = &stat[a + 1..b];
        if !(name.starts_with("tm-") || name.starts_with("crewhelm")) {
            continue;
        }
        let f: Vec<&str> = stat[b + 2..].split_whitespace().collect();
        let ticks = f.get(11).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0) + f.get(12).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let rss = kb_of(&read(&format!("/proc/{pid}/status")).await, "VmRSS:");
        out.push((pid, name.to_string(), ticks, rss));
    }
    out
}

async fn cmd(bin: &str, args: &[&str]) -> String {
    match tokio::time::timeout(Duration::from_secs(4), tokio::process::Command::new(bin).args(args).env("LC_ALL", "C").output()).await {
        Ok(Ok(o)) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => String::new(),
    }
}

fn db_path() -> String {
    let u = std::env::var("TM_DATABASE").unwrap_or_else(|_| "sqlite://tm-server.db".into());
    u.trim_start_matches("sqlite://").split('?').next().unwrap_or("").to_string()
}

async fn size_of(p: &str) -> u64 {
    tokio::fs::metadata(p).await.map(|m| m.len()).unwrap_or(0)
}

async fn procesos(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    need(&st, &user, false).await?;
    let a = snapshot().await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    let b = snapshot().await;
    let procs: Vec<Value> = b.iter().map(|(pid, name, t1, rss)| {
        let t0 = a.iter().find(|x| x.0 == *pid).map(|x| x.2).unwrap_or(*t1);
        json!({ "pid": pid, "name": name, "cpu": ((t1 - t0) as f64 / 100.0 / 0.6 * 100.0).round() / 1.0, "rss_mb": rss / 1024, "me": *pid == std::process::id() })
    }).collect();
    let mem = read("/proc/meminfo").await;
    let (total, avail) = (kb_of(&mem, "MemTotal:"), kb_of(&mem, "MemAvailable:"));
    let load: Vec<f64> = read("/proc/loadavg").await.split_whitespace().take(3).filter_map(|v| v.parse().ok()).collect();
    let up = read("/proc/uptime").await.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let cpus = read("/proc/cpuinfo").await.matches("processor").count().max(1);
    let path = db_path();
    let dir = std::path::Path::new(&path).parent().map(|p| p.to_string_lossy().to_string()).filter(|p| !p.is_empty()).unwrap_or_else(|| ".".into());
    let df = cmd("df", &["-Pk", &dir]).await;
    let disk: Vec<u64> = df.lines().nth(1).map(|l| l.split_whitespace().skip(1).take(2).filter_map(|v| v.parse().ok()).collect()).unwrap_or_default();
    let db_bytes = size_of(&path).await + size_of(&format!("{path}-wal")).await;
    let one = |q: &'static str| { let db = st.db.clone(); async move { sqlx::query_scalar::<_, i64>(q).fetch_one(&db).await.unwrap_or(0) } };
    let docker: Vec<Value> = cmd("docker", &["ps", "--format", "{{.Names}}|{{.Status}}|{{.Image}}"]).await.lines().filter_map(|l| {
        let p: Vec<&str> = l.split('|').collect();
        (p.len() == 3 && p[0].starts_with("crewhelm")).then(|| json!({ "name": p[0], "status": p[1], "image": p[2] }))
    }).collect();
    Ok(Json(json!({
        "procs": procs, "cpus": cpus, "load": load, "uptime_s": up as u64,
        "mem_mb": { "total": total / 1024, "used": total.saturating_sub(avail) / 1024 },
        "disk_mb": { "total": disk.first().copied().unwrap_or(0) / 1024, "used": disk.get(1).copied().unwrap_or(0) / 1024, "dir": dir },
        "app": {
            "db_mb": db_bytes / 1048576,
            "chat_files_mb": one("SELECT COALESCE(SUM(LENGTH(data)),0) FROM chat_files").await / 1048576,
            "users": one("SELECT COUNT(*) FROM users WHERE is_bot = 0 AND id NOT IN (SELECT user_id FROM api_tokens)").await,
            "ais": one("SELECT COUNT(DISTINCT user_id) FROM api_tokens").await,
            "teams": one("SELECT COUNT(*) FROM teams").await,
            "tasks": one("SELECT COUNT(*) FROM tasks").await,
            "tasks_doing": one("SELECT COUNT(*) FROM tasks WHERE status = 'doing'").await,
        },
        "docker": docker,
    })))
}
