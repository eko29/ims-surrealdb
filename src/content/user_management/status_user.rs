use actix_web::{web, HttpResponse};

use serde_json::json;
use surrealdb::sql::{Thing, Id};

use crate::{
    model_status::StatusBody, AppState,
    error_response::{ApiErrorType, ApiResponseError},
    model_status::DataJsonStatusUsers,
    sukses_response::ApiResponse,
    utilities::helpers::now_jakarta_time,
};

const TABLE_USER: &str = "m_user";

pub async fn update_status(
    info: web::Path<String>,
    app_state: web::Data<AppState>,
    body: web::Json<StatusBody>
) -> Result<HttpResponse, actix_web::Error> {
    
    let db = app_state.db.clone();
    let user_id_str = info.into_inner();
    let info_id = Thing::from((TABLE_USER, Id::String(user_id_str.clone())));
    let is_delete = false;

    let body_updated_at = now_jakarta_time();
    let status = body.status.clone();
    let reason = body.reason.clone();

    println!("info_id: {}", info_id);
    println!("nama table: {}", TABLE_USER);
    println!("status: {}", status);

    // Cek apakah user dengan ID tersebut ada dan belum dihapus
    let result = db
        .query("SELECT * FROM m_user WHERE id = $info_id AND is_delete = $is_delete LIMIT 1")
        .bind(("info_id", info_id.clone()))
        .bind(("is_delete", is_delete))
        .await;

    let users: Vec<DataJsonStatusUsers> = match result {
        Ok(mut query_result) => match query_result.take(0) {
            Ok(data) => data,
            Err(_) => {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    "Gagal mengambil data user".to_string(),
                );
                return Ok(HttpResponse::InternalServerError().json(err));
            }
        },
        Err(_) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                "Gagal menjalankan query cek user".to_string(),
            );
            return Ok(HttpResponse::InternalServerError().json(err));
        }
    };

    // Kalau user tidak ditemukan
    if users.is_empty() {
        let err = ApiResponseError::new(
            ApiErrorType::NotFound,
            "User tidak ditemukan".to_string(),
        );
        return Ok(HttpResponse::NotFound().json(err));
    }

    // Lanjut update user
    let update_result = db
        .query(
            r#"
            UPDATE m_user 
            SET updated_at = $body_updated_at, 
                status = $status,
                reason = $reason 
            WHERE id = $info_id AND is_delete = $is_delete RETURN AFTER
        "#,
        )
        .bind(("body_updated_at", body_updated_at))
        .bind(("status", status))
        .bind(("reason", reason))
        .bind(("info_id", info_id))
        .bind(("is_delete", is_delete))
        .await;

    match update_result {
        Ok(mut query_result) => {
            let updated_users: Vec<DataJsonStatusUsers> = match query_result.take(0) {
                Ok(data) => {
                    println!("Data hasil update: {:?}", data);
                    data
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal membaca hasil update: {}", e),
                    );
                    return Ok(HttpResponse::InternalServerError().json(err));
                }
            };
    
            if let Some(_user) = updated_users.into_iter().next() {
                let api_response = ApiResponse::new(
                    200,
                    "Sukses".to_string(),
                    json!({ "user": "Update berhasil" }),
                );
                Ok(HttpResponse::Ok().json(api_response))
            } else {
                let err = ApiResponseError::new(
                    ApiErrorType::NotFound,
                    "User tidak ditemukan".to_string(),
                );
                Ok(HttpResponse::NotFound().json(err))
            }
        }
        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Database error: {}", e),
            );
            Ok(HttpResponse::InternalServerError().json(err))
        }
    }
    
}