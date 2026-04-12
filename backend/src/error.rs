use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::collections::HashMap;

pub enum AppError {
    Validation(HashMap<String, String>),
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::Validation(fields) => (
                StatusCode::BAD_REQUEST,
                json!({ "error": "validation failed", "fields": fields }),
            ),
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": "unauthorized" }))
            }
            AppError::Forbidden => (StatusCode::FORBIDDEN, json!({ "error": "forbidden" })),
            AppError::NotFound => (StatusCode::NOT_FOUND, json!({ "error": "not found" })),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, json!({ "error": msg })),
            AppError::Internal(msg) => {
                tracing::error!("internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal server error" }),
                )
            }
        };

        (status, axum::Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound,
            sqlx::Error::Database(db_err) => {
                if db_err.constraint().is_some() {
                    AppError::Conflict("resource already exists".to_string())
                } else {
                    AppError::Internal(db_err.to_string())
                }
            }
            _ => AppError::Internal(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
