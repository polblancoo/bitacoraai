mod auth;
mod bots;
mod manual_img;
mod error;
mod canvas;
mod license;
mod license_token;
mod pro;
mod review;
mod limits;
mod notify;
mod extras;
mod github;
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
    pub mailer: Option<Arc<notify::Mailer>>,
    pub email_limiter: Arc<limits::RateLimiter<Id>>,
    pub telegram: Option<Arc<telegram::Telegram>>,
    pub google: Option<Arc<google::Google>>,
    pub github: Option<Arc<github::Github>>,
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
        "ALTER TABLE tasks ADD COLUMN parent_id INTEGER REFERENCES tasks(id) ON DELETE CASCADE",
        "ALTER TABLE team_members ADD COLUMN role TEXT NOT NULL DEFAULT 'member'",
        "ALTER TABLE users ADD COLUMN email TEXT",
        "ALTER TABLE activity ADD COLUMN task_id INTEGER",
        "ALTER TABLE projects ADD COLUMN require_review INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN email_code_hash TEXT",
        "ALTER TABLE users ADD COLUMN email_code_expires TEXT",
        "ALTER TABLE tasks ADD COLUMN start TEXT",
        "ALTER TABLE users ADD COLUMN telegram_chat_id INTEGER",
        "ALTER TABLE users ADD COLUMN tg_code_hash TEXT",
        "ALTER TABLE users ADD COLUMN tg_code_expires TEXT",
        "ALTER TABLE tasks ADD COLUMN recurrence TEXT",
        "ALTER TABLE tasks ADD COLUMN remind_state INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE team_members ADD COLUMN joined_at TEXT",
        "ALTER TABLE team_members ADD COLUMN invited_by INTEGER",
        "ALTER TABLE team_members ADD COLUMN via TEXT",
        "ALTER TABLE team_members ADD COLUMN invite_id INTEGER",
        "ALTER TABLE teams ADD COLUMN description TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE tasks ADD COLUMN created_by INTEGER",
        "ALTER TABLE projects ADD COLUMN work_dir TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE projects ADD COLUMN repo TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN model TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN is_bot INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE team_members ADD COLUMN executor INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN deny TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE users ADD COLUMN allow TEXT NOT NULL DEFAULT ''",
        "ALTER TABLE ssh_targets ADD COLUMN agent_id INTEGER",
        "ALTER TABLE projects ADD COLUMN token_quota INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE teams ADD COLUMN token_quota INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN token_budget INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE tasks ADD COLUMN position REAL NOT NULL DEFAULT 0",
        "ALTER TABLE report_subs ADD COLUMN via_telegram INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE report_subs ADD COLUMN via_email INTEGER NOT NULL DEFAULT 1",
    ] {
        let _ = sqlx::query(alter).execute(db).await;
    }
    // las tareas existentes conservan su orden de creación
    sqlx::query("UPDATE tasks SET position = id WHERE position = 0").execute(db).await?;
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
        mailer: notify::Mailer::from_env().map(Arc::new),
        email_limiter: Default::default(),
        telegram: telegram::Telegram::from_env().await.map(Arc::new),
        google: google::Google::from_env().map(Arc::new),
        github: github::Github::from_env().map(Arc::new),
    };
    telegram::spawn_poller(state.clone());
    extras::spawn_reminders(state.clone());
    google::spawn_sync(state.clone());
    license::spawn_ping(state.clone());
    schedule::spawn_schedules(state.clone());
    runner::spawn_runners(state.clone());
    report::spawn_reports(state.clone());

    let api = hardening(routes::router().merge(profile::router()).merge(canvas::router()).merge(extras::router()).merge(google::router()).merge(github::router()).merge(teampanel::router()).merge(bots::router()).merge(term::router()).merge(usage::router()).merge(license::router()).merge(pro::router()).merge(review::router()).merge(schedule::router()).merge(runner::router()).merge(report::router()).merge(ws::router()));
    let api = if web::enabled() { api.merge(web::router()) } else { api };
    let app: Router = api
        .layer(DefaultBodyLimit::max(limits::MAX_BODY_BYTES))
        .layer(cors())
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let svc = app.into_make_service_with_connect_info::<SocketAddr>();

    match (std::env::var("TM_TLS_CERT"), std::env::var("TM_TLS_KEY")) {
        (Ok(cert), Ok(key)) => {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            let cfg = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key).await?;
            tracing::info!("tm-server (HTTPS) escuchando en {addr}");
            axum_server::bind_rustls(addr, cfg).serve(svc).await?;
        }
        _ => {
            if !addr.ip().is_loopback() && !trust_proxy {
                tracing::warn!("sirviendo HTTP plano en {addr}: configurá TM_TLS_CERT/TM_TLS_KEY o un proxy TLS (TM_TRUSTED_PROXY=1)");
            }
            let listener = tokio::net::TcpListener::bind(addr).await?;
            tracing::info!("tm-server escuchando en {addr}");
            axum::serve(listener, svc).await?;
        }
    }
    Ok(())
}
