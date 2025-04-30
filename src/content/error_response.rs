use actix_web::{error::ResponseError, HttpResponse};
use actix_web::http::StatusCode;
use serde::Serialize;
use std::fmt;

// Enum untuk jenis error
#[derive(Debug, Serialize)]
pub enum ApiErrorType {
    BadRequest,
    NotFound,
    InternalServerError,
    Unauthorized,
    NoContent,
    Conflict,
    EnvVarError,
    ConnectionError,
    AuthError,
    DbSelectionError,
}

impl ApiErrorType {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiErrorType::BadRequest => StatusCode::BAD_REQUEST,
            ApiErrorType::NotFound => StatusCode::NOT_FOUND,
            ApiErrorType::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiErrorType::NoContent => StatusCode::NO_CONTENT,
            ApiErrorType::Conflict => StatusCode::CONFLICT,
            ApiErrorType::EnvVarError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::ConnectionError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorType::AuthError => StatusCode::UNAUTHORIZED,
            ApiErrorType::DbSelectionError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            ApiErrorType::BadRequest => "Invalid request parameters",
            ApiErrorType::NotFound => "Requested resource not found",
            ApiErrorType::InternalServerError => "An unexpected error occurred",
            ApiErrorType::Unauthorized => "No Authorization",
            ApiErrorType::NoContent => "No content available",
            ApiErrorType::Conflict => "Already exists",
            
            ApiErrorType::EnvVarError => "Gagal membaca variabel lingkungan: {0}",
            ApiErrorType::ConnectionError => "Gagal terhubung ke SurrealDB: {0}",
            ApiErrorType::AuthError => "Gagal autentikasi ke SurrealDB: {0}",
            ApiErrorType::DbSelectionError => "Gagal memilih namespace/database: {0}",
        }
    }
}

// Struct untuk response error
#[derive(Debug, Serialize)]
pub struct ApiResponseError {
    pub code: u16,
    pub response: String,
    pub message: String,
}


// ✅ Implementasikan std::fmt::Display
impl fmt::Display for ApiResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.response, self.message)
    }
}

// ✅ Implementasikan ResponseError
impl ResponseError for ApiResponseError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(self)
    }
}

// ✅ Menambahkan Fungsi `new`
impl ApiResponseError {
    pub fn new(error_type: ApiErrorType, detail: String) -> Self {
        ApiResponseError {
            code: error_type.status_code().as_u16(),
            response: format!("{:?}", error_type),
            message: detail,
        }
    }
}
