mod auth;
mod bots;
mod manual_img;
mod error;
mod canvas;
mod license;
mod license_token;
mod pro;
mod profile_pack;
mod digest;
mod review;
mod files;
mod taskfiles;
mod integrations;
mod chat_tr;
mod sysops;
mod chat;
mod chat_ai;
mod flow;
mod tls;
mod limits;
mod notify;
mod extras;
mod crypt;
mod github;
mod gh_sync;
mod gitsetup;
mod mailcfg;
mod setup;
mod prs;
mod reviews;
mod google;
mod teampanel;
mod telegram;
mod web;
mod profile;
mod routes;
mod report;
mod runner;
mod schedule;
mod term;
mod usage;
mod audit;
mod risk;
mod dav;
mod twofa;
mod gitmode;
mod ws;

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, HeaderValue, Method},
    Router,
};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::{net::{IpAddr, SocketAddr}, str::FromStr, sync::Arc};
use tm_core::{Event, Id};
use tokio::sync::broadcast;
use tower_http::{cors::CorsLayer, set_header::SetResponseHeaderLayer, trace::TraceLayer};

/// Evento ruteado a los miembros del equipo `team_id`.
#[derive(Clone)]
pub struct Envelope {
    pub team_id: Id,
    /// Si está presente, el evento es solo para ese usuario (y `team_id` se ignora).
    pub to_user: Option<Id>,
    pub event: Event,
}

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub secret: Arc<str>,
    pub tx: broadcast::Sender<Envelope>,
    pub ip_limiter: Arc<limits::RateLimiter<IpAddr>>,
    pub ip_limiter_ws: Arc<limits::RateLimiter<IpAddr>>,
    pub user_limiter: Arc<limits::RateLimiter<String>>,
    pub ws_conns: Arc<limits::WsCounter>,
    pub trust_proxy: bool,
    /// El propio servidor sirve HTTPS (TM_TLS_CERT / TM_TLS_KEY).
    pub tls: bool,
    pub mailer: notify::MailerSlot,
    pub email_limiter: Arc<limits::RateLimiter<Id>>,
    pub telegram: notify::Slot<telegram::Telegram>,
    pub google: notify::Slot<google::Google>,
    pub github: notify::Slot<github::Github>,
    /// «Todo en el servidor»: el administrador cortó GitHub (nada del código sale de acá). Ver gitmode.rs.
    pub git_internal: Arc<std::sync::atomic::AtomicBool>,
    pub cred: Arc<crypt::Cred>,
}

impl AppState {
    pub fn emit(&self, team_id: Id, event: Event) {
        let _ = self.tx.send(Envelope { team_id, to_user: None, event });
    }

    pub fn emit_to(&self, user: Id, event: Event) {
        let _ = self.tx.send(Envelope { team_id: 0, to_user: Some(user), event });
    }
}

async fn jwt_secret(db: &SqlitePool) -> anyhow::Result<String> {
    if let Ok(s) = std::env::var("TM_JWT_SECRET") {
        anyhow::ensure!(s.len() >= 32, "TM_JWT_SECRET debe tener al menos 32 caracteres");
        return Ok(s);
    }
    if let Some(row) = sqlx::query("SELECT value FROM meta WHERE key = 'jwt_secret'").fetch_optional(db).await? {
        return Ok(row.get(0));
    }
    let secret = auth::random_hex(32);
    sqlx::query("INSERT INTO meta (key, value) VALUES ('jwt_secret', ?)").bind(&secret).execute(db).await?;
    Ok(secret)
}

/// Esquemas viejos: agrega columnas nuevas; el error "duplicate column" es esperado y se ignora.
async fn migrate(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql(include_str!("schema.sql")).execute(db).await?;
    for alter in [
        "ALTER TABLE users ADD COLUMN token_version INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE reviews ADD COLUMN task_id INTEGER",
        "ALTER TABLE users ADD COLUMN avatar TEXT",
        "ALTER TABLE tasks ADD COLUMN parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE",
        "ALTER TABLE team_members ADD COLUMN role TEXT NOT NULL DEFAULT 'member'",
        "ALTER TABLE users ADD COLUMN email TEXT",
        "ALTER TABLE activity ADD COLUMN task_id INTEGER",
        "ALTER TABLE projects ADD COLUMN require_review INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE projects ADD COLUMN is_example INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE teams ADD COLUMN is_example INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE notifications ADD COLUMN ext_pending INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE notif_prefs ADD COLUMN every_h INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE notif_prefs ADD COLUMN mute_until TIMESTAMP",
        "ALTER TABLE users ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN email_code_hash TEXT",
        "ALTER TABLE users ADD COLUMN email_code_expires TEXT",
        "ALTER TABLE tasks ADD COLUMN start TEXT",
        "ALTER TABLE users ADD COLUMN telegram_chat_id INTEGER",
        "ALTER TABLE users ADD COLUMN tg_code_hash TEXT",
        "ALTER TABLE users ADD COLUMN tg_code_expires TEXT",
        "ALTER TABLE tasks ADD COLUMN recurrence TEXT",
        "ALTER TABLE tasks ADD COLUMN waiting INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE projects ADD COLUMN hold_ai_done INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE tasks ADD COLUMN remind_state INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE team_members ADD COLUMN joined_at TEXT",
        "ALTER TABLE team_members ADD COLUMN invited_by INTEGER",
        "ALTER TABLE team_members ADD COLUMN via TEXT",
        "ALTER TABLE team_members ADD COLUMN invite_id INTEGER",
        "ALTER TABLE teams ADD COLUMN description TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE tasks ADD COLUMN created_by INTEGER",
        "ALTER TABLE users ADD COLUMN full_access INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN totp_secret TEXT",
        "ALTER TABLE users ADD COLUMN totp_enabled INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN totp_last INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN dav_token_hash TEXT",
        "ALTER TABLE users ADD COLUMN base_allow TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN base_deny TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN lists_from_app INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE task_github ADD COLUMN pr_number INTEGER",
        "ALTER TABLE task_github ADD COLUMN pr_state TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE task_github ADD COLUMN last_gh_comment INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE task_github ADD COLUMN last_pushed_comment INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE projects ADD COLUMN work_dir TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE projects ADD COLUMN repo TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN model TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN is_bot INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE flow_steps ADD COLUMN par INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE flow_template_steps ADD COLUMN par INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN server_role TEXT NOT NULL DEFAULT 'user'",
        "ALTER TABLE team_members ADD COLUMN executor INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN deny TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN allow TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN isolation TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN term_prefs TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE ssh_targets ADD COLUMN agent_id INTEGER",
        "ALTER TABLE projects ADD COLUMN token_quota INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE teams ADD COLUMN token_quota INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN token_budget INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN position REAL NOT NULL DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE report_subs ADD COLUMN via_telegram INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE report_subs ADD COLUMN via_email INTEGER NOT NULL DEFAULT 1",
    ] {
        let _ = sqlx::query(alter).execute(db).await;
    }
    // las tareas existentes conservan su orden de creación
    sqlx::query("UPDATE tasks SET position = id WHERE position = 0").execute(db).await?;
    // si todavía no hay quien administre el servidor, la primera persona registrada lo pasa a ser
    sqlx::query("UPDATE users SET server_role = 'admin' WHERE id = (SELECT MIN(id) FROM users WHERE is_bot = 0 AND id NOT IN (SELECT user_id FROM api_tokens)) AND NOT EXISTS (SELECT 1 FROM users WHERE server_role = 'admin')").execute(db).await?;
    // el dueño de cada equipo siempre tiene rol owner (idempotente)
    sqlx::query("UPDATE team_members SET role = 'owner' WHERE role != 'owner' AND user_id = (SELECT owner_id FROM teams WHERE id = team_id)")
        .execute(db)
        .await?;
    Ok(())
}

/// Orígenes permitidos por CORS: los de Tauri, localhost y los de `TM_CORS_ORIGINS` (separados por coma).
fn cors() -> CorsLayer {
    let mut origins: Vec<HeaderValue> = [
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
        "http://localhost:1420",
        "http://127.0.0.1:1420",
    ]
    .iter()
    .map(|o| HeaderValue::from_static(o))
    .collect();
    if let Ok(extra) = std::env::var("TM_CORS_ORIGINS") {
        origins.extend(extra.split(',').filter_map(|o| HeaderValue::from_str(o.trim()).ok()));
    }
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(600))
}

/// Defensa básica contra saturación: tope de pedidos por IP y por minuto, tope de pedidos en simultáneo (rechaza con 503 en vez de encolar)
/// y límite de tiempo por pedido. Un ataque de red a gran escala se frena afuera (firewall, proxy o CDN); esto protege a la app de abusos simples.
static FLOOD: std::sync::LazyLock<limits::RateLimiter<IpAddr>> = std::sync::LazyLock::new(Default::default);
static INFLIGHT: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(400);
const REQ_PER_MIN: u32 = 3000;

async fn flood_guard(
    axum::extract::State(st): axum::extract::State<AppState>,
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<SocketAddr>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let ip = limits::client_ip(req.headers(), peer, st.trust_proxy);
    if FLOOD.hit(&ip, REQ_PER_MIN, std::time::Duration::from_secs(60)).is_err() {
        return (axum::http::StatusCode::TOO_MANY_REQUESTS, [(header::RETRY_AFTER, "30")], "demasiados pedidos, esperá un momento").into_response();
    }
    let Ok(_permit) = INFLIGHT.try_acquire() else {
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, [(header::RETRY_AFTER, "5")], "servidor ocupado, probá de nuevo en unos segundos").into_response();
    };
    next.run(req).await
}

fn hardening<T: Clone + Send + Sync + 'static>(router: Router<T>) -> Router<T> {
    let set = |name: HeaderName, value: &'static str| SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value));
    router
        .layer(set(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(set(header::CACHE_CONTROL, "no-store"))
        .layer(set(header::X_FRAME_OPTIONS, "DENY"))
        .layer(set(header::REFERRER_POLICY, "no-referrer"))
        .layer(set(header::CONTENT_SECURITY_POLICY, "default-src 'none'; frame-ancestors 'none'"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    // lettre (aws-lc-rs) y reqwest (ring) conviven: se fija un proveedor único para todo el proceso
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let db_url = std::env::var("TM_DATABASE").unwrap_or_else(|_| "sqlite://tm-server.db".into());
    let addr: SocketAddr = std::env::var("TM_ADDR").unwrap_or_else(|_| "0.0.0.0:7878".into()).parse()?;
    let trust_proxy = std::env::var("TM_TRUSTED_PROXY").is_ok_and(|v| v == "1");

    let opts = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let db = SqlitePool::connect_with(opts).await?;
    migrate(&db).await?;
    #[cfg(unix)]
    if let Some(path) = db_url.strip_prefix("sqlite://") {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    let secret = jwt_secret(&db).await?;
    let cred = Arc::new(crypt::init(&db).await?);
    let (tx, _) = broadcast::channel(256);
    let state = AppState {
        db,
        secret: secret.into(),
        tx,
        ip_limiter: Default::default(),
        ip_limiter_ws: Default::default(),
        user_limiter: Default::default(),
        ws_conns: Default::default(),
        trust_proxy,
        tls: std::env::var("TM_TLS_CERT").is_ok() && std::env::var("TM_TLS_KEY").is_ok(),
        mailer: notify::MailerSlot::new(notify::Mailer::from_env().map(Arc::new)),
        email_limiter: Default::default(),
        telegram: notify::Slot::new(telegram::Telegram::from_env().await.map(Arc::new)),
        google: notify::Slot::new(google::Google::from_env().map(Arc::new)),
        github: notify::Slot::new(github::Github::from_env().map(Arc::new)),
        git_internal: Default::default(),
        cred,
    };
    mailcfg::load(&state).await;
    gitmode::load(&state).await;
    setup::load(&state).await;
    telegram::spawn_poller(state.clone());
    extras::spawn_reminders(state.clone());
    google::spawn_sync(state.clone());
    gh_sync::spawn(state.clone());
    license::spawn_ping(state.clone());
    schedule::spawn_schedules(state.clone());
    runner::spawn_runners(state.clone());
    report::spawn_reports(state.clone());
    digest::spawn(state.clone());

    let api = hardening(routes::router().merge(profile::router()).merge(canvas::router()).merge(extras::router()).merge(google::router()).merge(github::router()).merge(gh_sync::router()).merge(gitsetup::router())
        .merge(prs::router())
        .merge(mailcfg::router()).merge(setup::router()).merge(teampanel::router()).merge(bots::router()).merge(term::router()).merge(usage::router()).merge(audit::router()).merge(risk::router()).merge(tls::router()).merge(twofa::router()).merge(gitmode::router()).merge(license::router()).merge(pro::router()).merge(profile_pack::router()).merge(digest::router()).merge(review::router()).merge(files::router()).merge(taskfiles::router()).merge(integrations::router()).merge(chat_tr::router()).merge(sysops::router()).merge(chat::router()).merge(chat_ai::router()).merge(flow::router()).merge(schedule::router()).merge(runner::router()).merge(report::router()).merge(ws::router()));
    let api = if web::enabled() { api.merge(web::router()) } else { api };
    let app: Router = api
        .layer(DefaultBodyLimit::max(limits::MAX_BODY_BYTES))
        .layer(tower_http::timeout::TimeoutLayer::with_status_code(axum::http::StatusCode::REQUEST_TIMEOUT, std::time::Duration::from_secs(120)))
        .layer(axum::middleware::from_fn_with_state(state.clone(), flood_guard))
        .layer(cors())
        .layer(TraceLayer::new_for_http())
        // la carpeta de red va aparte: sin tope de cuerpo ni de tiempo (son archivos grandes) y con su propio control de intentos
        .merge(dav::router())
        .with_state(state);
    match (std::env::var("TM_TLS_CERT"), std::env::var("TM_TLS_KEY")) {
        (Ok(cert), Ok(key)) => {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            let cfg = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!("tm-server (HTTPS) escuchando en {addr}");
            axum_server::bind_rustls(addr, cfg).serve(app.into_make_service_with_connect_info::<SocketAddr>()).await?;
        }
        _ => {
            // HTTP de siempre, más un HTTPS con certificado propio en el puerto siguiente (lo necesita el chat)
            if let Some(https_addr) = tls::plan(addr) {
                match tls::ensure_cert() {
                    Ok((cert, key)) => {
                        let svc = app.clone().layer(axum::middleware::from_fn(tls::mark)).into_make_service_with_connect_info::<SocketAddr>();
                        tokio::spawn(async move {
                            match axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await {
                                Ok(cfg) => {
                                    tracing::info!("tm-server (HTTPS automático) escuchando en {https_addr}");
                                    if let Err(e) = axum_server::bind_rustls(https_addr, cfg).serve(svc).await {
                                        tracing::warn!("el HTTPS automático se detuvo: {e}");
                                    }
                                }
                                Err(e) => tracing::warn!("no se pudo cargar el certificado propio: {e}"),
                            }
                        });
                    }
                    Err(e) => tracing::warn!("no se pudo crear el certificado HTTPS propio: {e}"),
                }
            }
            let svc = app.layer(axum::middleware::from_fn(tls::strip)).into_make_service_with_connect_info::<SocketAddr>();
            let listener = tokio::net::TcpListener::bind(addr).await?;
            tracing::info!("tm-server escuchando en {addr}");
            axum::serve(listener, svc).await?;
        }
    }
    Ok(())
}
