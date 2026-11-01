//! Agente integrado: el servidor arranca y supervisa el `tm-agent` de las IAs que una persona activó desde la web,
//! para no pedir comandos en una terminal. Solo la primera persona registrada (admin de la instancia) lo maneja.
//! Corre en la misma máquina que el servidor; las claves de las IAs se toman del entorno del servidor
//! (`ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, `OPENAI_API_KEY`) y no se guardan. `TM_RUNNER=0` lo desactiva.
use crate::{
    auth::AuthUser,
    bots::store_token,
    error::{AppError, Result},
    license::instance_admin,
    AppState,
};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::{Duration, Instant}};
use tm_core::Id;
use tokio::process::{Child, Command};

const RUN_CLAUDE: &str = include_str!("../assets/agents/run-claude.sh");
const WHITELIST: &str = include_str!("../assets/agents/whitelist.txt");
const BLACKLIST: &str = include_str!("../assets/agents/blacklist.txt");

#[derive(Clone, Default)]
struct Info {
    pid: Option<u32>,
    since: Option<String>,
    last_exit: Option<String>,
}

fn infos() -> &'static Mutex<HashMap<Id, Info>> {
    static M: std::sync::OnceLock<Mutex<HashMap<Id, Info>>> = std::sync::OnceLock::new();
    M.get_or_init(Default::default)
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/bots/{uid}/runner", get(get_runner).put(put_runner)).route("/api/runner", get(available)).route("/api/runner/openrouter-key", axum::routing::put(put_openrouter_key))
}

fn enabled_by_env() -> bool {
    std::env::var("TM_RUNNER").map_or(true, |v| v != "0")
}

fn agent_bin() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("tm-agent"))).filter(|p| p.exists()).unwrap_or_else(|| "tm-agent".into())
}

/// PATH del servidor más los lugares donde suelen quedar las herramientas de IA (npm con nvm, cargo, ~/.local/bin):
/// un servicio systemd arranca sin el PATH del shell y no las encontraría.
fn tool_path() -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".cargo/bin"));
        if let Ok(rd) = std::fs::read_dir(home.join(".nvm/versions/node")) {
            let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path().join("bin")).collect();
            v.sort();
            dirs.extend(v.into_iter().rev());
        }
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

fn which(bin: &str) -> bool {
    std::env::split_paths(&tool_path()).any(|d| d.join(bin).exists())
}

/// Clave de OpenRouter: archivo propio del servidor (permisos 600), nunca en la base de datos.
fn openrouter_file() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from).join(".config/tm-server/openrouter.env")
}

fn openrouter_key() -> Option<String> {
    let txt = std::fs::read_to_string(openrouter_file()).ok();
    txt.and_then(|t| t.lines().find_map(|l| l.trim().strip_prefix("OPENROUTER_API_KEY=").map(|v| v.trim().trim_matches('"').to_string())))
        .filter(|k| !k.is_empty())
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok().filter(|k| !k.is_empty()))
}

#[derive(Deserialize)]
struct KeyBody {
    key: String,
}

async fn put_openrouter_key(State(st): State<AppState>, user: AuthUser, Json(b): Json<KeyBody>) -> Result<Json<Value>> {
    guard(&st, &user).await?;
    let k = b.key.trim();
    if k.len() < 10 || k.len() > 200 || !k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        return Err(AppError::bad("clave inválida (solo letras, números y - _ .)"));
    }
    let path = openrouter_file();
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| AppError::bad(&format!("no pude guardar la clave: {e}")))?;
    }
    std::fs::write(&path, format!("OPENROUTER_API_KEY={k}\n")).map_err(|e| AppError::bad(&format!("no pude guardar la clave: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(Json(json!({ "ok": true })))
}

fn data_dir() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from).join(".local/share/tm-server/agents")
}

async fn available(State(st): State<AppState>, user: AuthUser) -> Result<Json<Value>> {
    let admin = !user.bot && instance_admin(&st.db, user.id).await;
    let bin = agent_bin();
    let found = bin.is_absolute() || which("tm-agent");
    Ok(Json(json!({
        "allowed": admin && enabled_by_env() && cfg!(unix), "agent_found": found,
        "tools": { "claude": which("claude"), "gemini": which("gemini"), "openai": which("codex"), "openrouter": which("opencode") },
        "openrouter_key": openrouter_key().is_some(),
    })))
}

async fn guard(st: &AppState, user: &AuthUser) -> Result<()> {
    if user.bot || !enabled_by_env() || !instance_admin(&st.db, user.id).await {
        return Err(AppError::forbidden());
    }
    Ok(())
}

async fn get_runner(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>) -> Result<Json<Value>> {
    guard(&st, &user).await?;
    let row = sqlx::query("SELECT kind, model, url, enabled FROM bot_runners WHERE user_id = ?").bind(uid).fetch_optional(&st.db).await?;
    let info = infos().lock().map(|m| m.get(&uid).cloned().unwrap_or_default()).unwrap_or_default();
    Ok(Json(match row {
        None => json!({ "configured": false }),
        Some(r) => json!({
            "configured": true, "kind": r.get::<String, _>("kind"), "model": r.get::<String, _>("model"), "url": r.get::<String, _>("url"),
            "enabled": r.get::<i64, _>("enabled") != 0, "running": info.pid.is_some(), "since": info.since, "last_exit": info.last_exit,
        }),
    }))
}

#[derive(Deserialize)]
struct Body {
    kind: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    url: String,
    enabled: bool,
}

async fn put_runner(State(st): State<AppState>, user: AuthUser, Path(uid): Path<Id>, Json(b): Json<Body>) -> Result<Json<Value>> {
    guard(&st, &user).await?;
    if !matches!(b.kind.as_str(), "claude" | "gemini" | "openai" | "local" | "openrouter") {
        return Err(AppError::bad("tipo de IA no soportado para ejecutarse en el servidor"));
    }
    let model = b.model.trim();
    // el modelo y la dirección terminan en una línea de comandos: solo caracteres inofensivos
    let safe = |s: &str, extra: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c));
    if model.is_empty() || model.len() > 90 || !safe(model, ".:_-/") {
        return Err(AppError::bad("modelo inválido (letras, números y . : _ - /)"));
    }
    if b.url.len() > 200 || !safe(&b.url, ".:_-/@%") || (b.kind == "local" && !b.url.starts_with("http")) {
        return Err(AppError::bad("dirección del servidor de IA inválida (debe empezar con http)"));
    }
    let is_bot: Option<i64> = sqlx::query_scalar("SELECT is_bot FROM users WHERE id = ?").bind(uid).fetch_optional(&st.db).await?;
    if is_bot != Some(1) {
        return Err(AppError::not_found());
    }
    sqlx::query(
        "INSERT INTO bot_runners (user_id, kind, model, url, enabled, set_by) VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(user_id) DO UPDATE SET kind = excluded.kind, model = excluded.model, url = excluded.url, enabled = excluded.enabled, set_by = excluded.set_by",
    )
    .bind(uid)
    .bind(&b.kind)
    .bind(model)
    .bind(&b.url)
    .bind(i64::from(b.enabled))
    .bind(user.id)
    .execute(&st.db)
    .await?;
    Ok(Json(json!({ "ok": true })))
}

fn port() -> u16 {
    std::env::var("TM_ADDR").ok().and_then(|a| a.rsplit(':').next().and_then(|p| p.parse().ok())).unwrap_or(7878)
}

/// Comando y variables de entorno según el tipo de IA. Claude usa el script con permisos que se pide por la web (🙋).
fn plan(kind: &str, model: &str, url: &str, conf: &std::path::Path) -> (String, Vec<(String, String)>) {
    match kind {
        "claude" => (
            format!("'{}'", conf.join("run-claude.sh").to_string_lossy()),
            vec![("TM_CLAUDE_MODEL".into(), model.to_lowercase()), ("TM_AGENT_CONF".into(), conf.to_string_lossy().into_owned())],
        ),
        "gemini" => (format!("gemini -p \"$TM_TASK_PROMPT\" -m {model}"), vec![]),
        "openrouter" => (
            format!("opencode run -m openrouter/{model} \"$TM_TASK_PROMPT\""),
            openrouter_key().map(|k| vec![("OPENROUTER_API_KEY".to_string(), k)]).unwrap_or_default(),
        ),
        "openai" => (format!("codex exec \"$TM_TASK_PROMPT\" -m {model}"), vec![]),
        _ => (
            format!("codex exec \"$TM_TASK_PROMPT\" -m {model}"),
            vec![("OPENAI_BASE_URL".into(), url.into()), ("OPENAI_API_KEY".into(), std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sin-clave".into()))],
        ),
    }
}

fn prepare_conf(name: &str) -> std::io::Result<PathBuf> {
    let dir = data_dir().join(name);
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("run-claude.sh");
        std::fs::write(&script, RUN_CLAUDE)?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))?;
    }
    // las listas las edita la persona: solo se crean si no existen
    for (f, c) in [("whitelist.txt", WHITELIST), ("blacklist.txt", BLACKLIST)] {
        if !dir.join(f).exists() {
            std::fs::write(dir.join(f), c)?;
        }
    }
    Ok(dir)
}

async fn spawn_one(st: &AppState, uid: Id, by: Id, name: &str, kind: &str, model: &str, url: &str) -> std::io::Result<Child> {
    let conf = if kind == "claude" { prepare_conf(name)? } else { data_dir() };
    let (cmd, env) = plan(kind, model, url, &conf);
    // cada arranque emite un token nuevo (el anterior deja de valer): no hace falta guardarlo
    let token = store_token(st, uid, by).await.map_err(|_| std::io::Error::other("no se pudo emitir el token"))?;
    let mut c = Command::new(agent_bin());
    c.env("TM_URL", format!("http://127.0.0.1:{}", port()))
        .env("TM_TOKEN", token)
        .env("TM_AGENT_CMD", cmd)
        .env("TM_AGENT_POLL", "10")
        .envs(env)
        .env("PATH", tool_path())
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    c.spawn()
}

pub fn spawn_runners(st: AppState) {
    if !enabled_by_env() || !cfg!(unix) {
        return;
    }
    tokio::spawn(async move {
        let mut kids: HashMap<Id, (Child, String)> = HashMap::new();
        let mut cooldown: HashMap<Id, Instant> = HashMap::new();
        // al cortar el servidor (systemctl stop, Ctrl+C) se cierran también los agentes: si no, quedan huérfanos con un token vencido
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).ok();
        loop {
            let stop = async {
                match (term.as_mut(), int.as_mut()) {
                    (Some(t), Some(i)) => tokio::select! { _ = t.recv() => {}, _ = i.recv() => {} },
                    _ => std::future::pending().await,
                }
            };
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                _ = stop => {
                    for (_, (c, _)) in kids.iter_mut() {
                        let _ = c.start_kill();
                    }
                    std::process::exit(0);
                }
            }
            let rows = sqlx::query(
                "SELECT r.user_id, r.kind, r.model, r.url, r.set_by, u.username FROM bot_runners r JOIN users u ON u.id = r.user_id WHERE r.enabled = 1",
            )
            .fetch_all(&st.db)
            .await
            .unwrap_or_default();
            let wanted: HashMap<Id, (String, String, String, Id, String)> = rows
                .iter()
                .map(|r| (r.get("user_id"), (r.get("kind"), r.get("model"), r.get("url"), r.get::<Option<Id>, _>("set_by").unwrap_or(0), r.get("username"))))
                .collect();
            // parar las desactivadas o cuya configuración cambió
            let stale: Vec<Id> = kids
                .iter()
                .filter(|(id, (_, sig))| wanted.get(id).map(|w| format!("{}|{}|{}", w.0, w.1, w.2)) != Some(sig.clone()))
                .map(|(id, _)| *id)
                .collect();
            for id in stale {
                if let Some((mut c, _)) = kids.remove(&id) {
                    let _ = c.kill().await;
                }
                if let Ok(mut m) = infos().lock() {
                    m.insert(id, Info { last_exit: Some("detenida".into()), ..Default::default() });
                }
            }
            // detectar caídas
            let mut dead = vec![];
            for (id, (c, _)) in kids.iter_mut() {
                if let Ok(Some(status)) = c.try_wait() {
                    dead.push((*id, status.to_string()));
                }
            }
            for (id, why) in dead {
                kids.remove(&id);
                cooldown.insert(id, Instant::now());
                if let Ok(mut m) = infos().lock() {
                    m.insert(id, Info { last_exit: Some(why), ..Default::default() });
                }
            }
            // arrancar las que faltan
            for (id, (kind, model, url, by, name)) in wanted {
                if kids.contains_key(&id) || cooldown.get(&id).is_some_and(|t| t.elapsed() < Duration::from_secs(30)) {
                    continue;
                }
                match spawn_one(&st, id, by, &name, &kind, &model, &url).await {
                    Ok(c) => {
                        if let Ok(mut m) = infos().lock() {
                            m.insert(id, Info { pid: c.id(), since: Some(chrono::Utc::now().to_rfc3339()), last_exit: None });
                        }
                        kids.insert(id, (c, format!("{kind}|{model}|{url}")));
                    }
                    Err(e) => {
                        tracing::warn!("agente integrado de {name}: {e}");
                        cooldown.insert(id, Instant::now());
                        if let Ok(mut m) = infos().lock() {
                            m.insert(id, Info { last_exit: Some(format!("no arrancó: {e}")), ..Default::default() });
                        }
                    }
                }
            }
        }
    });
}
