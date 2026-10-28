use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};

#[derive(Debug)]
pub struct AppError(pub StatusCode, pub String);

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn bad(msg: &str) -> Self { Self(StatusCode::BAD_REQUEST, msg.into()) }
    pub fn unauthorized(msg: &str) -> Self { Self(StatusCode::UNAUTHORIZED, msg.into()) }
    pub fn forbidden() -> Self { Self(StatusCode::FORBIDDEN, "sin acceso".into()) }
    pub fn not_found() -> Self { Self(StatusCode::NOT_FOUND, "no encontrado".into()) }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::not_found(),
            e => {
                tracing::error!("db: {e}");
                Self(StatusCode::INTERNAL_SERVER_ERROR, "error interno".into())
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}
