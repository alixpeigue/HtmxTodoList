use axum::{http::StatusCode, response::IntoResponse};

pub enum ApplicationError {
    NotFound,
    InternalError(String),
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApplicationError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_owned()),
            ApplicationError::InternalError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        }
        .into_response()
    }
}

impl From<anyhow::Error> for ApplicationError {
    fn from(value: anyhow::Error) -> Self {
        Self::InternalError(value.to_string())
    }
}
