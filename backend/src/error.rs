use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use mysqlview_types::ApiError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Db(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "BAD_REQUEST",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Db(_) => "DB_ERROR",
            AppError::Internal(_) => "INTERNAL",
        }
    }

    fn hint(&self) -> Option<String> {
        match self {
            AppError::Db(sqlx::Error::Database(db_err)) => db_err.code().map(|c| c.into_owned()),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if matches!(self, AppError::Db(_) | AppError::Internal(_)) {
            tracing::error!(error = %self, "request failed");
        } else {
            tracing::debug!(error = %self, "request rejected");
        }

        let body = ApiError {
            code: self.code().to_owned(),
            message: self.to_string(),
            hint: self.hint(),
        };
        (self.status(), Json(body)).into_response()
    }
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;
