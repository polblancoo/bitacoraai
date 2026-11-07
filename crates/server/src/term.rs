//! Terminal integrada: el navegador habla por WebSocket con el servidor, y este abre una sesión SSH
//! (con clave, sin contraseñas) a la máquina del proyecto. Solo owners. La clave privada queda en un
//! archivo del servidor (`~/.config/tm-server/ssh/`, modo 600): nunca viaja ni se guarda en la base.
use crate::{auth::authenticate, error::{AppError, Result}, limits, routes, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Path, Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use russh::{client, keys, ChannelMsg, Disconnect};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use sqlx::Row;
use tm_core::{Id, Role};

/// Terminales abiertas por persona (el multiplexor abre varias a la vez; cada una es una tarea async independiente).
static OPEN: Mutex<Option<HashMap<Id, usize>>> = Mutex::new(None);
const MAX_PER_USER: usize = 8;

/// Terminal por agente: el `tm-agent` de la máquina del proyecto abre un shell local y lo conecta al servidor
/// (conexión saliente: sin claves, sin puertos abiertos). Aquí se empareja con el navegador.
struct AgentEnd {
    from_browser: mpsc::Receiver<Message>,
    to_browser: mpsc::Sender<Message>,
}
struct Pending {
    agent: Id,
    dir: String,
    cols: u32,
    rows: u32,
    /// quién abre la terminal y si es owner (el agente decide el HOME y si va dentro del sandbox)
    who: String,
    owner: bool,
    /// espacio personal ajeno que un owner mira en solo lectura (auditoría)
    view: Option<String>,
    handoff: Option<AgentEnd>,
}
static PENDING: LazyLock<Mutex<HashMap<String, Pending>>> = LazyLock::new(Default::default);
/// Última vez que cada agente con terminal habilitada consultó al servidor.
static AGENTS: LazyLock<Mutex<HashMap<Id, (Instant, bool, bool)>>> = LazyLock::new(Default::default);
const AGENT_ONLINE: Duration = Duration::from_secs(15);

fn agent_online(id: Id) -> bool {
    AGENTS.lock().unwrap().get(&id).is_some_and(|t| t.0.elapsed() < AGENT_ONLINE)
}
const IDLE: Duration = Duration::from_secs(30 * 60);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/projects/{id}/ssh", get(get_target).put(put_target))
        .route("/api/projects/{id}/ssh/keygen", post(keygen))
        .route("/api/projects/{id}/ssh/forget-host", post(forget_host))
        .route("/api/projects/{id}/ssh/copy-all", post(copy_all))
        .route("/api/projects/{id}/ssh/install-key", post(install_key))
        .route("/api/projects/{id}/term", get(handler))
        .route("/api/projects/{id}/term/agents", get(list_agents))
        .route("/api/me/term-prefs", get(get_term_prefs).put(put_term_prefs))
        .route("/api/me/ssh-keys", get(list_ssh_keys).post(add_ssh_key))
        .route("/api/me/ssh-keys/{id}", axum::routing::delete(del_ssh_key))
        .route("/api/agent/ssh-keys", get(agent_ssh_keys))
        .route("/api/projects/{id}/term/settings", get(get_term_settings).put(put_term_settings))
        .route("/api/agent/term/config", get(agent_term_config))
        .route("/api/agent/term/pending", get(agent_pending))
        .route("/api/agent/term/{sid}", get(agent_ws))
}

fn key_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config/tm-server/ssh")
}

fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(rest),
        None => PathBuf::from(p),
    }
}

async fn team_of_project(st: &AppState, project: Id) -> Result<Id> {
    sqlx::query_scalar("SELECT team_id FROM projects WHERE id = ?").bind(project).fetch_optional(&st.db).await?.ok_or_else(AppError::forbidden)
}

fn public_key(path: &str) -> Option<String> {
    let s = std::fs::read_to_string(format!("{}.pub", expand(path).display())).ok()?;
    Some(s.trim().to_string())
}

async fn get_target(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    let row: Option<(String, i64, String, String, String, Option<i64>)> =
        sqlx::query_as("SELECT host, port, user, key_path, host_fp, agent_id FROM ssh_targets WHERE project_id = ?").bind(project).fetch_optional(&st.db).await?;
    Ok(Json(match row {
        Some((host, port, u, key_path, fp, agent)) => json!({ "host": host, "port": port, "user": u, "key_path": key_path, "host_fp": fp, "agent_id": agent, "public_key": public_key(&key_path) }),
        None => json!({ "host": "", "port": 22, "user": "", "key_path": format!("~/.config/tm-server/ssh/proyecto-{project}"), "host_fp": "", "agent_id": null, "public_key": null }),
    }))
}

#[derive(Deserialize)]
struct Target {
    #[serde(default)]
    host: String,
    port: Option<u16>,
    #[serde(default)]
    user: String,
    key_path: Option<String>,
    /// Si viene, la terminal de este proyecto sale por ese agente (sin SSH).
    agent_id: Option<Id>,
}

async fn put_target(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>, Json(t): Json<Target>) -> Result<StatusCode> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    if let Some(agent) = t.agent_id {
        let team = team_of_project(&st, project).await?;
        let ok: Option<i64> = sqlx::query_scalar("SELECT 1 FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND m.user_id = ? AND u.is_bot = 1")
            .bind(team)
            .bind(agent)
            .fetch_optional(&st.db)
            .await?;
        ok.ok_or_else(|| AppError::bad("ese agente no es una IA de este equipo"))?;
        sqlx::query(
            "INSERT INTO ssh_targets (project_id, host, port, user, key_path, host_fp, agent_id) VALUES (?1, '', 22, '', '', '', ?2) \
             ON CONFLICT(project_id) DO UPDATE SET agent_id = ?2",
        )
        .bind(project)
        .bind(agent)
        .execute(&st.db)
        .await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let (host, u) = (t.host.trim(), t.user.trim());
    let ok = |s: &str| !s.is_empty() && s.len() <= 120 && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-:".contains(c));
    if !ok(host) || !ok(u) {
        return Err(AppError::bad("host o usuario inválido (letras, números, . _ - :)"));
    }
    let key_path = t.key_path.filter(|k| !k.trim().is_empty()).unwrap_or_else(|| format!("~/.config/tm-server/ssh/proyecto-{project}"));
    limits::max_len("ruta de la clave", &key_path, 300)?;
    // si cambia la máquina, hay que volver a confiar en su huella
    sqlx::query(
        "INSERT INTO ssh_targets (project_id, host, port, user, key_path, host_fp) VALUES (?1, ?2, ?3, ?4, ?5, '') \
         ON CONFLICT(project_id) DO UPDATE SET host_fp = CASE WHEN host = ?2 AND port = ?3 THEN host_fp ELSE '' END, host = ?2, port = ?3, user = ?4, key_path = ?5, agent_id = NULL",
    )
    .bind(project)
    .bind(host)
    .bind(i64::from(t.port.unwrap_or(22)))
    .bind(u)
    .bind(key_path.trim())
    .execute(&st.db)
    .await?;
    // con la ruta por defecto la clave se crea sola; la pública se ve en la ventana para pegarla en la máquina
    let path = expand(key_path.trim());
    if path.starts_with(key_dir()) {
        let _ = ensure_key(&path, project).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Genera un par ed25519 sin passphrase en el servidor y devuelve la clave pública para `authorized_keys`.
async fn keygen(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    let key_path: String = sqlx::query_scalar("SELECT key_path FROM ssh_targets WHERE project_id = ?")
        .bind(project)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| AppError::bad("guardá primero host y usuario"))?;
    ensure_key(&expand(&key_path), project).await.map_err(|e| AppError::bad(&e))?;
    Ok(Json(json!({ "public_key": public_key(&key_path) })))
}

/// Usa la misma máquina SSH (host, usuario, clave y huella) en todos los proyectos del equipo.
async fn copy_all(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = team_of_project(&st, project).await?;
    routes::require(&st, team, user.id, Role::Owner).await?;
    let n = sqlx::query(
        "INSERT INTO ssh_targets (project_id, host, port, user, key_path, host_fp, agent_id) \
         SELECT p.id, t.host, t.port, t.user, t.key_path, t.host_fp, t.agent_id FROM projects p, ssh_targets t WHERE t.project_id = ?1 AND p.team_id = ?2 AND p.id != ?1 \
         ON CONFLICT(project_id) DO UPDATE SET host = excluded.host, port = excluded.port, user = excluded.user, key_path = excluded.key_path, host_fp = excluded.host_fp, agent_id = excluded.agent_id",
    )
    .bind(project)
    .bind(team)
    .execute(&st.db)
    .await?
    .rows_affected();
    Ok(Json(json!({ "copied": n })))
}

/// Crea el par ed25519 (sin passphrase) si todavía no existe.
async fn ensure_key(path: &std::path::Path, project: Id) -> std::result::Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().unwrap_or(&key_dir())).map_err(|e| format!("no pude crear la carpeta de claves: {e}"))?;
    let out = tokio::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", &format!("tm-server proyecto {project}"), "-f"])
        .arg(path)
        .output()
        .await
        .map_err(|e| format!("no pude ejecutar ssh-keygen: {e}"))?;
    if out.status.success() { Ok(()) } else { Err("ssh-keygen falló".into()) }
}

#[derive(Deserialize)]
struct InstallKey {
    password: String,
}

/// Como `ssh-copy-id`: entra una sola vez con la contraseña de la máquina y deja la clave pública en `authorized_keys`.
/// La contraseña se usa en esta llamada y se descarta: no se guarda ni se registra.
async fn install_key(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>, Json(b): Json<InstallKey>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    st.user_limiter.hit(&format!("sshpw:{}", user.id), 5, Duration::from_secs(60)).map_err(|_| AppError::bad("demasiados intentos: esperá un minuto"))?;
    limits::max_len("contraseña", &b.password, 200)?;
    let (host, port, ssh_user, key_path): (String, i64, String, String) = sqlx::query_as("SELECT host, port, user, key_path FROM ssh_targets WHERE project_id = ?")
        .bind(project)
        .fetch_optional(&st.db)
        .await?
        .ok_or_else(|| AppError::bad("guardá primero máquina y usuario"))?;
    let path = expand(&key_path);
    ensure_key(&path, project).await.map_err(|e| AppError::bad(&e))?;
    let pubkey = public_key(&key_path).filter(|k| k.starts_with("ssh-ed25519 ") && !k.contains(['\'', '\n', '\r', '\\'])).ok_or_else(|| AppError::bad("no pude leer la clave pública"))?;
    let seen = Arc::new(Mutex::new(None));
    let cfg = Arc::new(client::Config { inactivity_timeout: Some(Duration::from_secs(30)), ..Default::default() });
    let mut ssh = tokio::time::timeout(Duration::from_secs(15), client::connect(cfg, (host.as_str(), port as u16), Client { db: st.db.clone(), project, seen: seen.clone() }))
        .await
        .map_err(|_| AppError::bad(&format!("{host}:{port} no responde en 15 s: revisá la IP, el servicio SSH y el firewall")))?
        .map_err(|e| AppError::bad(&format!("no pude conectar con {host}:{port}: {e}")))?;
    let auth = ssh.authenticate_password(ssh_user.clone(), b.password).await.map_err(|e| AppError::bad(&e.to_string()))?;
    if !auth.success() {
        return Err(AppError::bad("la máquina rechazó el usuario o la contraseña (o no acepta contraseñas por SSH)"));
    }
    let mut ch = ssh.channel_open_session().await.map_err(|e| AppError::bad(&e.to_string()))?;
    let cmd = format!("umask 077; mkdir -p ~/.ssh && touch ~/.ssh/authorized_keys && {{ grep -qxF '{pubkey}' ~/.ssh/authorized_keys || echo '{pubkey}' >> ~/.ssh/authorized_keys; }} && chmod 700 ~/.ssh && chmod 600 ~/.ssh/authorized_keys");
    ch.exec(true, cmd).await.map_err(|e| AppError::bad(&e.to_string()))?;
    let mut code = None;
    while let Some(m) = tokio::time::timeout(Duration::from_secs(15), ch.wait()).await.ok().flatten() {
        if let ChannelMsg::ExitStatus { exit_status } = m {
            code = Some(exit_status);
            break;
        }
    }
    let _ = ssh.disconnect(Disconnect::ByApplication, "", "es").await;
    if code != Some(0) {
        return Err(AppError::bad("no pude escribir ~/.ssh/authorized_keys en esa máquina (¿es Linux/macOS? en Windows hay que hacerlo a mano)"));
    }
    let fp = sqlx::query_scalar::<_, String>("SELECT host_fp FROM ssh_targets WHERE project_id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    tracing::info!("terminal: usuario {} instaló la clave del proyecto {project} en {host}", user.id);
    Ok(Json(json!({ "ok": true, "host_fp": fp })))
}

async fn forget_host(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<StatusCode> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    sqlx::query("UPDATE ssh_targets SET host_fp = '' WHERE project_id = ?").bind(project).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct TermQuery {
    task: Option<Id>,
}

async fn handler(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(project): Path<Id>,
    Query(q): Query<TermQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let ip = limits::client_ip(&headers, peer, st.trust_proxy);
    if st.ip_limiter_ws.hit(&ip, 20, Duration::from_secs(60)).is_err() {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    ws.max_message_size(64 * 1024).max_frame_size(64 * 1024).on_upgrade(move |sock| session(sock, st, project, q.task))
}

struct Client {
    db: sqlx::SqlitePool,
    project: Id,
    /// Huella vista en esta conexión (para avisar al usuario si se la fijó por primera vez).
    seen: Arc<Mutex<Option<(String, bool)>>>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(&mut self, key: &russh::keys::PublicKeyOrCertificate) -> std::result::Result<bool, Self::Error> {
        let russh::keys::PublicKeyOrCertificate::PublicKey { key, .. } = key else { return Ok(false) };
        let fp = key.fingerprint(keys::HashAlg::Sha256).to_string();
        let known: String = sqlx::query_scalar("SELECT host_fp FROM ssh_targets WHERE project_id = ?").bind(self.project).fetch_one(&self.db).await.unwrap_or_default();
        let first = known.is_empty();
        if !first && known != fp {
            *self.seen.lock().unwrap() = Some((fp, false));
            return Ok(false);
        }
        if first {
            let _ = sqlx::query("UPDATE ssh_targets SET host_fp = ? WHERE project_id = ?").bind(&fp).bind(self.project).execute(&self.db).await;
        }
        *self.seen.lock().unwrap() = Some((fp, first));
        Ok(true)
    }
}

async fn say(sock: &mut WebSocket, v: Value) {
    let _ = sock.send(Message::Text(v.to_string().into())).await;
}

async fn session(mut sock: WebSocket, st: AppState, project: Id, task: Option<Id>) {
    // primer mensaje: {"token": "...", "cols": 80, "rows": 24}
    let first = tokio::time::timeout(Duration::from_secs(5), sock.recv()).await.ok().flatten().and_then(|m| m.ok());
    let Some(Message::Text(text)) = first else { return };
    let Ok(v) = serde_json::from_str::<Value>(&text) else { return };
    let user = match v.get("token").and_then(Value::as_str) {
        Some(t) => authenticate(&st, t).await,
        None => None,
    };
    let (cols, rows) = (v["cols"].as_u64().unwrap_or(80).clamp(10, 500) as u32, v["rows"].as_u64().unwrap_or(24).clamp(2, 200) as u32);
    let Some(user) = user.filter(|u| !u.bot) else {
        say(&mut sock, json!({ "error": "sin acceso" })).await;
        return;
    };
    let team = match team_of_project(&st, project).await {
        Ok(t) => t,
        Err(_) => return say(&mut sock, json!({ "error": "sin acceso" })).await,
    };
    let agent: Option<i64> = sqlx::query_scalar("SELECT agent_id FROM ssh_targets WHERE project_id = ?").bind(project).fetch_optional(&st.db).await.ok().flatten().flatten();
    // owners: terminal completa; miembros: solo por un agente con espacios personales y sandbox (TM_WORKSPACE), nunca por SSH
    if routes::require(&st, team, user.id, Role::Owner).await.is_err() {
        let sandboxed = agent.is_some_and(|a| AGENTS.lock().unwrap().get(&a).is_some_and(|t| t.2));
        if !(sandboxed && routes::require(&st, team, user.id, Role::Member).await.is_ok()) {
            return say(&mut sock, json!({ "error": "la terminal es solo para owners (los miembros la usan cuando el agente del proyecto tiene espacios personales: TM_WORKSPACE)" })).await;
        }
    }
    let fresh = {
        let mut g = OPEN.lock().unwrap();
        let n = g.get_or_insert_with(HashMap::new).entry(user.id).or_insert(0);
        (*n < MAX_PER_USER).then(|| *n += 1).is_some()
    };
    if !fresh {
        return say(&mut sock, json!({ "error": format!("máximo {MAX_PER_USER} terminales abiertas a la vez") })).await;
    }
    let started = Instant::now();
    let view = v.get("space").and_then(Value::as_str).map(str::trim).filter(|x| !x.is_empty() && x.len() <= 64).map(String::from);
    if view.is_some() && routes::require(&st, team, user.id, Role::Owner).await.is_err() {
        return say(&mut sock, json!({ "error": "ver el espacio de otra persona es solo para owners" })).await;
    }
    let res = match agent {
        Some(a) => run_agent(&mut sock, &st, project, task, user.id, cols, rows, a, view).await,
        None => run(&mut sock, &st, project, task, user.id, cols, rows).await,
    };
    if let Some(n) = OPEN.lock().unwrap().get_or_insert_with(HashMap::new).get_mut(&user.id) {
        *n = n.saturating_sub(1);
    }
    let host = res.as_ref().ok().cloned().unwrap_or_default();
    if let Err(e) = &res {
        say(&mut sock, json!({ "error": e })).await;
    }
    let what = format!("terminal en {host} durante {} s{}", started.elapsed().as_secs(), res.as_ref().err().map(|e| format!(" (error: {e})")).unwrap_or_default());
    if let Some(task) = task {
        crate::extras::log_task(&st, task, user.id, what.clone()).await;
    }
    tracing::info!("terminal: usuario {} · proyecto {project} · {what}", user.id);
    let _ = sock.send(Message::Close(None)).await;
}

/// Abre el SSH y reenvía datos en ambos sentidos. Devuelve el host al que se conectó.
async fn run(sock: &mut WebSocket, st: &AppState, project: Id, task: Option<Id>, who: Id, cols: u32, rows: u32) -> std::result::Result<String, String> {
    let (host, port, user, key_path): (String, i64, String, String) = sqlx::query_as("SELECT host, port, user, key_path FROM ssh_targets WHERE project_id = ?")
        .bind(project)
        .fetch_optional(&st.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("este proyecto no tiene conexión configurada: usá el botón «⚙ Conexión» de la vista Terminales (o «💻 Terminal» dentro de una tarea) para cargar máquina, usuario y clave")?;
    let key = keys::load_secret_key(expand(&key_path), None).map_err(|e| format!("todavía no hay una clave utilizable en {key_path} ({e}). Tocá «🔑 Generar clave» y pegá la línea pública en ~/.ssh/authorized_keys de la máquina"))?;
    let seen = Arc::new(Mutex::new(None));
    let cfg = Arc::new(client::Config { inactivity_timeout: Some(IDLE), keepalive_interval: Some(Duration::from_secs(30)), ..Default::default() });
    let handler = Client { db: st.db.clone(), project, seen: seen.clone() };
    let mut ssh = tokio::time::timeout(Duration::from_secs(15), client::connect(cfg, (host.as_str(), port as u16), handler))
        .await
        .map_err(|_| format!("{host}:{port} no responde en 15 s. Revisá que la IP sea la de la máquina DEL PROYECTO, que tenga un servidor SSH activo y que el firewall deje pasar el puerto"))?
        .map_err(|e| {
            if matches!(seen.lock().unwrap().as_ref(), Some((_, false))) {
                format!("¡La huella de {host} cambió! Puede ser otra máquina o un ataque. Si fue un cambio legítimo, usá «Olvidar huella».")
            } else {
                format!("no pude conectar con {host}:{port}: {e}")
            }
        })?;
    let first_seen = seen.lock().unwrap().clone();
    if let Some((fp, true)) = first_seen {
        say(sock, json!({ "info": format!("Primera conexión: guardé la huella de la máquina ({fp}). Verificala.") })).await;
    }
    let hash = ssh.best_supported_rsa_hash().await.map_err(|e| e.to_string())?.flatten();
    let auth = ssh.authenticate_publickey(user.clone(), keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash)).await.map_err(|e| e.to_string())?;
    if !auth.success() {
        return Err(format!("{host} rechazó la clave para el usuario {user}: agregá la clave pública a ~/.ssh/authorized_keys"));
    }
    let mut ch = ssh.channel_open_session().await.map_err(|e| e.to_string())?;
    ch.request_pty(false, "xterm-256color", cols, rows, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    ch.request_shell(true).await.map_err(|e| e.to_string())?;
    // arranca en la carpeta del proyecto si existe
    let work_dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    if !work_dir.is_empty() && !work_dir.contains(['\'', '\n', '\r', '\0']) {
        // «~/carpeta» entre comillas simples no se expande: se cambia por $HOME
        let cd = match work_dir.strip_prefix("~/") { Some(rest) => format!("cd \"$HOME\"/'{rest}'"), None if work_dir == "~" => "cd \"$HOME\"".into(), None => format!("cd '{work_dir}'") };
        let _ = ch.data_bytes(format!("{cd} 2>/dev/null && clear\n").into_bytes()).await;
    }
    let started = chrono::Utc::now();
    let mut transcript: Vec<u8> = Vec::new();
    let mut last = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(30));
    loop {
        tokio::select! {
            m = ch.wait() => match m {
                Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                    transcript.extend_from_slice(&data);
                    if transcript.len() > 400_000 { transcript.drain(..100_000); }
                    if sock.send(Message::Binary(data.to_vec().into())).await.is_err() { break; }
                }
                Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | Some(ChannelMsg::ExitStatus { .. }) | None => break,
                _ => {}
            },
            incoming = sock.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(Message::Binary(b))) => { last = Instant::now(); if ch.data_bytes(b.to_vec()).await.is_err() { break; } }
                Some(Ok(Message::Text(t))) => {
                    if let Some(sz) = serde_json::from_str::<Value>(&t).ok().and_then(|v| v.get("resize").cloned()) {
                        let (c, r) = (sz[0].as_u64().unwrap_or(80).clamp(10, 500) as u32, sz[1].as_u64().unwrap_or(24).clamp(2, 200) as u32);
                        let _ = ch.window_change(c, r, 0, 0).await;
                    }
                }
                _ => {}
            },
            _ = tick.tick() => if last.elapsed() > IDLE { break; },
        }
    }
    if let Some(path) = save_note(&mut ssh, st, project, task, who, &host, started, &transcript).await {
        if let Some(task) = task {
            crate::routes::system_comment(st, task, who, format!("📝 Guardé lo que se hizo en la terminal en `{path}` (la IA lo lee al retomar la tarea).")).await;
        }
    }
    let _ = ssh.disconnect(Disconnect::ByApplication, "", "es").await;
    Ok(host)
}

/// Quita secuencias de escape y caracteres de control para dejar el texto que se vio en pantalla.
fn clean(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let mut out = String::new();
    let mut it = text.chars();
    while let Some(c) = it.next() {
        match c {
            '\x1b' => match it.next() {
                Some('[') => { for n in it.by_ref() { if ('@'..='~').contains(&n) { break; } } }
                Some(']') => { for n in it.by_ref() { if n == '\x07' || n == '\x1b' { break; } } }
                _ => {}
            },
            '\x08' => { out.pop(); }
            '\r' => {}
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Arma la nota Markdown de la sesión: (carpeta del proyecto, ruta relativa, texto).
#[allow(clippy::too_many_arguments)]
async fn build_note(st: &AppState, project: Id, task: Option<Id>, who: Id, host: &str, started: chrono::DateTime<chrono::Utc>, raw: &[u8]) -> Option<(String, String, String)> {
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.ok()?;
    if dir.is_empty() || dir.contains(['\'', '\n', '\r', '\0']) {
        return None;
    }
    let screen = clean(raw);
    let screen = screen.trim();
    if screen.is_empty() {
        return None;
    }
    let chars: Vec<char> = screen.chars().collect();
    let tail: String = chars[chars.len().saturating_sub(12_000)..].iter().collect();
    let who_name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(who).fetch_one(&st.db).await.unwrap_or_default();
    let note = format!(
        "\n## Sesión de terminal · {} · {who_name} en {host}\n\n{}. Lo que se vio en pantalla (comandos y salidas; las contraseñas no se muestran). Seguí desde acá.\n\n```text\n{}\n```\n",
        started.format("%Y-%m-%d %H:%M UTC"),
        task.map_or_else(|| "Sin tarea".to_string(), |t| format!("Tarea #{t}")),
        tail.replace("```", "\'\'\'")
    );
    let rel = task.map_or_else(|| ".tm/terminal-proyecto.md".to_string(), |t| format!(".tm/terminal-tarea-{t}.md"));
    Some((dir, rel, note))
}

/// Guarda la nota en la máquina SSH (`.tm/terminal-tarea-N.md` dentro de la carpeta del proyecto).
#[allow(clippy::too_many_arguments)]
async fn save_note(ssh: &mut client::Handle<Client>, st: &AppState, project: Id, task: Option<Id>, who: Id, host: &str, started: chrono::DateTime<chrono::Utc>, raw: &[u8]) -> Option<String> {
    let (dir, rel, note) = build_note(st, project, task, who, host, started, raw).await?;
    let mut ch = ssh.channel_open_session().await.ok()?;
    let cmd = format!("cd '{dir}' && mkdir -p .tm && cat >> '{rel}' && {{ [ ! -d .git ] || grep -qx '.tm/' .git/info/exclude 2>/dev/null || echo '.tm/' >> .git/info/exclude; }}");
    ch.exec(true, cmd).await.ok()?;
    ch.data_bytes(note.into_bytes()).await.ok()?;
    ch.eof().await.ok()?;
    let mut code = None;
    while let Some(m) = tokio::time::timeout(Duration::from_secs(10), ch.wait()).await.ok().flatten() {
        if let ChannelMsg::ExitStatus { exit_status } = m { code = Some(exit_status); break; }
    }
    (code == Some(0)).then(|| format!("{dir}/{rel}"))
}

// ---------- terminal por el agente (tm-agent con TM_AGENT_TERMINAL=1) ----------

/// Las IAs del equipo del proyecto y si su agente tiene la terminal habilitada y conectada.
async fn list_agents(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    let team = team_of_project(&st, project).await?;
    routes::require(&st, team, user.id, Role::Member).await?;
    let owner = routes::require(&st, team, user.id, Role::Owner).await.is_ok();
    let rows: Vec<(Id, String, String)> = sqlx::query_as("SELECT u.id, u.username, u.model FROM team_members m JOIN users u ON u.id = m.user_id WHERE m.team_id = ? AND u.is_bot = 1 ORDER BY u.username")
        .bind(team)
        .fetch_all(&st.db)
        .await?;
    Ok(Json(Value::Array(rows.into_iter().map(|(id, name, model)| json!({ "id": id, "name": name, "model": model, "online": agent_online(id), "herdr": AGENTS.lock().unwrap().get(&id).is_some_and(|t| t.1), "workspace": AGENTS.lock().unwrap().get(&id).is_some_and(|t| t.2) })).filter(|j| owner || j["workspace"] == true).collect())))
}

/// Preferencias de la terminal de cada persona (viajan con la cuenta, no con el navegador).
async fn get_term_prefs(State(st): State<AppState>, user: crate::auth::AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let raw: String = sqlx::query_scalar("SELECT term_prefs FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    Ok(Json(serde_json::from_str(&raw).unwrap_or_else(|_| json!({}))))
}

async fn put_term_prefs(State(st): State<AppState>, user: crate::auth::AuthUser, Json(b): Json<Value>) -> Result<Json<Value>> {
    user.human_only()?;
    // solo claves conocidas, valores simples
    let mut out = serde_json::Map::new();
    if let Some(v) = b.get("autoHerdr").and_then(Value::as_bool) {
        out.insert("autoHerdr".into(), json!(v));
    }
    let raw: String = sqlx::query_scalar("SELECT term_prefs FROM users WHERE id = ?").bind(user.id).fetch_one(&st.db).await?;
    let mut cur: serde_json::Map<String, Value> = serde_json::from_str(&raw).unwrap_or_default();
    cur.extend(out);
    sqlx::query("UPDATE users SET term_prefs = ? WHERE id = ?").bind(Value::Object(cur.clone()).to_string()).bind(user.id).execute(&st.db).await?;
    Ok(Json(Value::Object(cur)))
}

// ---------- ajustes de la terminal para las personas del equipo (los prende un owner desde la app, sin consola) ----------

fn flag_key(kind: &str, agent: Id) -> String {
    format!("term_{kind}_{agent}")
}

async fn flag(st: &AppState, kind: &str, agent: Id) -> bool {
    crate::license::meta_get(&st.db, &flag_key(kind, agent)).await.is_some_and(|v| v == "1")
}

async fn project_agent(st: &AppState, project: Id) -> Option<Id> {
    sqlx::query_scalar("SELECT agent_id FROM ssh_targets WHERE project_id = ?").bind(project).fetch_optional(&st.db).await.ok().flatten().flatten()
}

async fn get_term_settings(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    let agent = project_agent(&st, project).await;
    let (people, ssh) = match agent { Some(a) => (flag(&st, "people", a).await, flag(&st, "ssh", a).await), None => (false, false) };
    Ok(Json(json!({ "agent_id": agent, "people": people, "ssh": ssh })))
}

#[derive(Deserialize)]
struct TermSettings {
    people: bool,
    #[serde(default)]
    ssh: bool,
}

async fn put_term_settings(State(st): State<AppState>, user: crate::auth::AuthUser, Path(project): Path<Id>, Json(b): Json<TermSettings>) -> Result<Json<Value>> {
    user.human_only()?;
    routes::require(&st, team_of_project(&st, project).await?, user.id, Role::Owner).await?;
    let agent = project_agent(&st, project).await.ok_or_else(|| AppError::bad("primero elegí el agente de la máquina y conectá"))?;
    // la entrada por SSH usa los espacios personales: sin permitir la terminal a las personas no tiene sentido
    let ssh = b.ssh && b.people;
    crate::license::meta_set(&st.db, &flag_key("people", agent), if b.people { "1" } else { "0" }).await?;
    crate::license::meta_set(&st.db, &flag_key("ssh", agent), if ssh { "1" } else { "0" }).await?;
    Ok(Json(json!({ "agent_id": agent, "people": b.people, "ssh": ssh })))
}

/// Lo que el agente necesita saber para activarse solo: si algún proyecto lo usa como terminal, y si abre espacios para las personas / entrada por SSH.
async fn agent_term_config(State(st): State<AppState>, user: crate::auth::AuthUser) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    let used: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ssh_targets WHERE agent_id = ?").bind(user.id).fetch_one(&st.db).await?;
    let (people, ssh) = (flag(&st, "people", user.id).await, flag(&st, "ssh", user.id).await);
    Ok(Json(json!({ "terminal": used > 0, "people": people, "ssh": people && ssh })))
}

// ---------- claves SSH de las personas (entrada por SSH al usuario de servicio, con su espacio personal) ----------

/// Deja solo `tipo base64` de una línea pública (sin opciones ni comentario, que podrían cambiar lo que permite la clave).
fn clean_pubkey(line: &str) -> Option<String> {
    let mut it = line.split_whitespace();
    let (kind, body) = (it.next()?, it.next()?);
    let kinds = ["ssh-ed25519", "ssh-rsa", "ecdsa-sha2-nistp256", "ecdsa-sha2-nistp384", "ecdsa-sha2-nistp521", "sk-ssh-ed25519@openssh.com"];
    let b64 = body.len() >= 40 && body.len() <= 800 && body.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='));
    (kinds.contains(&kind) && b64).then(|| format!("{kind} {body}"))
}

async fn list_ssh_keys(State(st): State<AppState>, user: crate::auth::AuthUser) -> Result<Json<Value>> {
    user.human_only()?;
    let rows: Vec<(Id, String, String, String)> = sqlx::query_as("SELECT id, key, label, created_at FROM user_ssh_keys WHERE user_id = ? ORDER BY id").bind(user.id).fetch_all(&st.db).await?;
    Ok(Json(Value::Array(rows.into_iter().map(|(id, key, label, at)| json!({ "id": id, "label": label, "created_at": at, "fingerprint": format!("{}…{}", &key[..key.find(' ').map_or(0, |i| i + 9).min(key.len())], &key[key.len().saturating_sub(10)..]) })).collect())))
}

#[derive(Deserialize)]
struct NewKey {
    key: String,
    #[serde(default)]
    label: String,
}

async fn add_ssh_key(State(st): State<AppState>, user: crate::auth::AuthUser, Json(b): Json<NewKey>) -> Result<Json<Value>> {
    user.human_only()?;
    let key = clean_pubkey(&b.key).ok_or_else(|| AppError(StatusCode::BAD_REQUEST, "no parece una clave pública SSH (empieza con ssh-ed25519, ssh-rsa o ecdsa-sha2-…)".into()))?;
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_ssh_keys WHERE user_id = ?").bind(user.id).fetch_one(&st.db).await?;
    if n >= 5 {
        return Err(AppError(StatusCode::BAD_REQUEST, "máximo 5 claves por persona: borrá una que ya no uses".into()));
    }
    let label: String = b.label.chars().filter(|c| !c.is_control()).take(60).collect();
    sqlx::query("INSERT OR IGNORE INTO user_ssh_keys (user_id, key, label) VALUES (?, ?, ?)").bind(user.id).bind(&key).bind(label.trim()).execute(&st.db).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn del_ssh_key(State(st): State<AppState>, user: crate::auth::AuthUser, Path(id): Path<Id>) -> Result<StatusCode> {
    user.human_only()?;
    sqlx::query("DELETE FROM user_ssh_keys WHERE id = ? AND user_id = ?").bind(id).bind(user.id).execute(&st.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Las claves de las personas de los equipos de este agente, con su nivel: `owner` si lo es en alguno de esos equipos, si no `member`.
async fn agent_ssh_keys(State(st): State<AppState>, user: crate::auth::AuthUser) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT u.username, k.key, MAX(CASE WHEN m.role = 'owner' THEN 1 ELSE 0 END) \
         FROM user_ssh_keys k JOIN users u ON u.id = k.user_id AND u.is_bot = 0 \
         JOIN team_members m ON m.user_id = u.id \
         WHERE m.team_id IN (SELECT team_id FROM team_members WHERE user_id = ?) \
         GROUP BY k.id ORDER BY k.id",
    )
    .bind(user.id)
    .fetch_all(&st.db)
    .await?;
    Ok(Json(Value::Array(rows.into_iter().map(|(who, key, o)| json!({ "who": who, "key": key, "level": if o == 1 { "owner" } else { "member" } })).collect())))
}

/// El agente pregunta cada segundo si hay una terminal para abrir (así también se sabe que está conectado).
async fn agent_pending(user: crate::auth::AuthUser, Query(q): Query<HashMap<String, String>>) -> Result<Json<Value>> {
    if !user.bot {
        return Err(AppError::forbidden());
    }
    // el agente avisa si la máquina tiene herdr instalado (la web propone instalarlo al abrir la terminal)
    AGENTS.lock().unwrap().insert(user.id, (Instant::now(), q.get("herdr").is_some_and(|v| v == "1"), q.get("ws").is_some_and(|v| v == "1")));
    let list: Vec<Value> = PENDING
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, p)| p.agent == user.id && p.handoff.is_some())
        .map(|(sid, p)| json!({ "sid": sid, "dir": p.dir, "cols": p.cols, "rows": p.rows, "who": p.who, "owner": p.owner, "view": p.view }))
        .collect();
    Ok(Json(Value::Array(list)))
}

async fn agent_ws(State(st): State<AppState>, headers: HeaderMap, Path(sid): Path<String>, ws: WebSocketUpgrade) -> Response {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("").to_string();
    let Some(user) = authenticate(&st, &token).await.filter(|u| u.bot) else { return StatusCode::UNAUTHORIZED.into_response() };
    let end = {
        let mut g = PENDING.lock().unwrap();
        match g.get_mut(&sid) {
            Some(p) if p.agent == user.id => p.handoff.take(),
            _ => None,
        }
    };
    let Some(end) = end else { return StatusCode::NOT_FOUND.into_response() };
    ws.max_message_size(256 * 1024).max_frame_size(256 * 1024).on_upgrade(move |mut sock| async move {
        let AgentEnd { mut from_browser, to_browser } = end;
        loop {
            tokio::select! {
                m = from_browser.recv() => match m {
                    Some(m) => if sock.send(m).await.is_err() { break; },
                    None => break,
                },
                incoming = sock.recv() => match incoming {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    Some(Ok(m @ (Message::Binary(_) | Message::Text(_)))) => if to_browser.send(m).await.is_err() { break; },
                    _ => {}
                },
            }
        }
        let _ = sock.send(Message::Close(None)).await;
    })
}

/// Resumen de la tarea para una IA que se abre a mano en la terminal: qué hay que hacer y lo último que se habló.
async fn task_context(st: &AppState, task: Id) -> Option<String> {
    let r = sqlx::query(
        "SELECT t.title, t.description, t.status, t.priority, COALESCE(u.username, 'sin responsable') AS who, p.name AS project, p.repo, p.work_dir \
         FROM tasks t JOIN projects p ON p.id = t.project_id LEFT JOIN users u ON u.id = t.assignee_id WHERE t.id = ?",
    )
    .bind(task)
    .fetch_optional(&st.db)
    .await
    .ok()??;
    let comments: Vec<(String, String)> = sqlx::query_as(
        "SELECT u.username, c.body FROM comments c JOIN users u ON u.id = c.user_id WHERE c.task_id = ? ORDER BY c.id DESC LIMIT 12",
    )
    .bind(task)
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    let cut = |s: &str, n: usize| -> String { s.chars().take(n).collect() };
    let mut out = format!(
        "# Contexto de la tarea #{task}\n\n- Título: {}\n- Proyecto: {}\n- Repositorio: {}\n- Carpeta de trabajo: {}\n- Estado: {}\n- Responsable: {}\n\n## Qué hay que hacer\n\n{}\n",
        r.get::<String, _>("title"),
        r.get::<String, _>("project"),
        r.get::<String, _>("repo"),
        r.get::<String, _>("work_dir"),
        r.get::<String, _>("status"),
        r.get::<String, _>("who"),
        cut(&r.get::<String, _>("description"), 4000),
    );
    if !comments.is_empty() {
        out.push_str("\n## Lo último que se habló (del más nuevo al más viejo)\n\n");
        for (who, body) in &comments {
            out.push_str(&format!("- **{who}**: {}\n", cut(body, 600).replace('\n', " ")));
        }
    }
    out.push_str("\n## Cómo trabajar\n\nTrabajá dentro de esta carpeta. Si en `.tm/terminal-tarea-N.md` hay notas de sesiones anteriores, seguí desde ahí. Antes de ejecutar algo delicado (borrar, publicar, tocar servicios) preguntale a la persona que está en la terminal.\n");
    Some(out)
}

/// Sesión por agente: reenvía bytes entre el navegador y el shell que abre `tm-agent` en la máquina del proyecto.
#[allow(clippy::too_many_arguments)]
async fn run_agent(sock: &mut WebSocket, st: &AppState, project: Id, task: Option<Id>, who: Id, cols: u32, rows: u32, agent: Id, view: Option<String>) -> std::result::Result<String, String> {
    let name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(agent).fetch_one(&st.db).await.map_err(|e| e.to_string())?;
    if !agent_online(agent) {
        return Err(format!("el agente «{name}» no está conectado. En su máquina tiene que correr tm-agent con TM_AGENT_TERMINAL=1 (y TM_TOKEN de esa IA)."));
    }
    let dir: String = sqlx::query_scalar("SELECT work_dir FROM projects WHERE id = ?").bind(project).fetch_one(&st.db).await.unwrap_or_default();
    let sid = uuid::Uuid::new_v4().simple().to_string();
    let (to_agent, from_browser) = mpsc::channel::<Message>(512);
    let (to_browser, mut from_agent) = mpsc::channel::<Message>(512);
    let who_name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = ?").bind(who).fetch_one(&st.db).await.unwrap_or_default();
    let owner = match team_of_project(st, project).await { Ok(t) => routes::require(st, t, who, Role::Owner).await.is_ok(), Err(_) => false };
    PENDING.lock().unwrap().insert(sid.clone(), Pending { agent, dir, cols, rows, who: who_name, owner, view: view.clone(), handoff: Some(AgentEnd { from_browser, to_browser }) });
    // el contexto de la tarea queda escrito en la carpeta del proyecto, para que una IA abierta en esta terminal sepa de qué se trata
    if let Some(t) = task {
        if let Some(body) = task_context(st, t).await {
            let _ = to_agent.send(Message::Text(json!({ "note": { "rel": format!(".tm/terminal-contexto-{t}.md"), "body": body, "replace": true } }).to_string().into())).await;
        }
    }
    let started = chrono::Utc::now();
    let host = match &view { Some(v) => format!("espacio de {v} (solo lectura) por el agente {name}"), None => format!("agente {name}") };
    let mut transcript: Vec<u8> = Vec::new();
    let mut last = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    let connect_deadline = Instant::now() + Duration::from_secs(15);
    let mut err = None;
    loop {
        tokio::select! {
            m = from_agent.recv() => match m {
                Some(Message::Binary(b)) => {
                    transcript.extend_from_slice(&b);
                    if transcript.len() > 400_000 { transcript.drain(..100_000); }
                    // mientras la IA trabaja en la terminal hay salida: la sesión no se da por inactiva aunque nadie esté mirando
                    last = Instant::now();
                    if sock.send(Message::Binary(b)).await.is_err() { break; }
                }
                Some(Message::Text(t)) => if t.contains("\"exit\"") { break; },
                _ => break,
            },
            incoming = sock.recv() => match incoming {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(m @ Message::Binary(_))) => { last = Instant::now(); if to_agent.send(m).await.is_err() { break; } }
                Some(Ok(m @ Message::Text(_))) => { let _ = to_agent.send(m).await; }
                _ => {}
            },
            _ = tick.tick() => {
                let waiting = PENDING.lock().unwrap().get(&sid).is_some_and(|p| p.handoff.is_some());
                if waiting && Instant::now() > connect_deadline { err = Some(format!("el agente «{name}» no tomó la sesión (¿se cortó?)")); break; }
                if last.elapsed() > IDLE { break; }
            }
        }
    }
    PENDING.lock().unwrap().remove(&sid);
    if let Some((dir, rel, body)) = build_note(st, project, task, who, &host, started, &transcript).await {
        // la nota se escribe en la máquina del proyecto: la manda el servidor y la guarda el agente
        let _ = to_agent.send(Message::Text(json!({ "note": { "rel": rel, "body": body } }).to_string().into())).await;
        tokio::time::sleep(Duration::from_millis(800)).await;
        if let Some(task) = task {
            crate::routes::system_comment(st, task, who, format!("📝 Guardé lo que se hizo en la terminal en `{dir}/{rel}` (la IA lo lee al retomar la tarea).")).await;
        }
    }
    match err {
        Some(e) => Err(e),
        None => Ok(host),
    }
}

#[cfg(test)]
mod key_tests {
    use super::clean_pubkey;

    #[test]
    fn la_clave_publica_se_limpia_y_se_valida() {
        let body = "AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l8GKJl";
        assert_eq!(clean_pubkey(&format!("ssh-ed25519 {body} pablo@pc")).as_deref(), Some(&*format!("ssh-ed25519 {body}")));
        // opciones delante (command=, no-pty…) no son una clave: se rechaza
        assert!(clean_pubkey(&format!("command=\"rm -rf /\" ssh-ed25519 {body}")).is_none());
        assert!(clean_pubkey("ssh-ed25519 corta").is_none());
        assert!(clean_pubkey("ssh-dss AAAA").is_none());
    }
}
