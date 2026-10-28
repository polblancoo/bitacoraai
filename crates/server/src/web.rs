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
const CANVAS_JS: &str = include_str!("../web/canvas.js");
const STYLE: &str = include_str!("../web/style.css");
const MANIFEST: &str = include_str!("../web/manifest.webmanifest");
const SW: &str = include_str!("../web/sw.js");
const ICON_192: &[u8] = include_bytes!("../web/icon-192.png");
const ICON_512: &[u8] = include_bytes!("../web/icon-512.png");

fn bytes(mime: &'static str, body: &'static [u8]) -> Response {
    ([(header::CONTENT_TYPE, mime)], body).into_response()
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
        .route("/canvas.js", get(|| async { asset("text/javascript; charset=utf-8", CANVAS_JS) }))
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
