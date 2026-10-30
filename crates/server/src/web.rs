//! Cliente web: la misma interfaz gráfica del escritorio, embebida en el binario y servida en `/`.
//! Se desactiva con `TM_WEB=0`.
use axum::{
    http::{header, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tower_http::set_header::SetResponseHeaderLayer;

const INDEX: &str = include_str!("../web/index.html");
const APP_JS: &str = include_str!("../web/app.js");
const I18N_JS: &str = include_str!("../web/i18n.js");
const CANVAS_JS: &str = include_str!("../web/canvas.js");
const XTERM_JS: &str = include_str!("../web/xterm.js");
const XTERM_FIT: &str = include_str!("../web/xterm-fit.js");
const XTERM_CSS: &str = include_str!("../web/xterm.css");
const STYLE: &str = include_str!("../web/style.css");
const MANIFEST: &str = include_str!("../web/manifest.webmanifest");
const SW: &str = include_str!("../web/sw.js");
const MANUAL_ES: &str = include_str!("../web/manual/es.md");
const MANUAL_EN: &str = include_str!("../web/manual/en.md");
const MANUAL_PT: &str = include_str!("../web/manual/pt.md");
const ICON_192: &[u8] = include_bytes!("../web/icon-192.png");
const ICON_512: &[u8] = include_bytes!("../web/icon-512.png");

fn bytes(mime: &'static str, body: &'static [u8]) -> Response {
    ([(header::CONTENT_TYPE, mime)], body).into_response()
}

async fn manual_img(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    match crate::manual_img::IMGS.iter().find(|(n, _)| *n == name) {
        Some((_, b)) => ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=86400")], *b).into_response(),
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}

fn asset(mime: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, mime)], body).into_response()
}

pub fn enabled() -> bool {
    std::env::var("TM_WEB").map_or(true, |v| v != "0")
}

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let set = |name, value: &'static str| SetResponseHeaderLayer::overriding(name, HeaderValue::from_static(value));
    Router::new()
        .route("/", get(|| async { asset("text/html; charset=utf-8", INDEX) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }))
        .route("/manual/es.md", get(|| async { asset("text/markdown; charset=utf-8", MANUAL_ES) }))
        .route("/manual/en.md", get(|| async { asset("text/markdown; charset=utf-8", MANUAL_EN) }))
        .route("/manual/pt.md", get(|| async { asset("text/markdown; charset=utf-8", MANUAL_PT) }))
        .route("/manual/img/{name}", get(manual_img))
        .route("/i18n.js", get(|| async { asset("text/javascript; charset=utf-8", I18N_JS) }))
        .route("/canvas.js", get(|| async { asset("text/javascript; charset=utf-8", CANVAS_JS) }))
        .route("/xterm.js", get(|| async { asset("text/javascript; charset=utf-8", XTERM_JS) }))
        .route("/xterm-fit.js", get(|| async { asset("text/javascript; charset=utf-8", XTERM_FIT) }))
        .route("/xterm.css", get(|| async { asset("text/css; charset=utf-8", XTERM_CSS) }))
        .route("/style.css", get(|| async { asset("text/css; charset=utf-8", STYLE) }))
        .route("/manifest.webmanifest", get(|| async { asset("application/manifest+json", MANIFEST) }))
        .route("/sw.js", get(|| async { asset("text/javascript; charset=utf-8", SW) }))
        .route("/icon-192.png", get(|| async { bytes("image/png", ICON_192) }))
        .route("/icon-512.png", get(|| async { bytes("image/png", ICON_512) }))
        .layer(set(header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(set(header::X_FRAME_OPTIONS, "DENY"))
        .layer(set(header::REFERRER_POLICY, "no-referrer"))
        .layer(set(header::CACHE_CONTROL, "no-cache"))
        .layer(set(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self' ws: wss:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ))
}
