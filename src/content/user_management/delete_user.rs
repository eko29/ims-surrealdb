use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use surrealdb::sql::{Thing, Id};

use crate::{
    error_response::{ApiErrorType, ApiResponseError}, model_user::DataJsonUsers, sukses_response::ApiResponse, AppState
};

pub async fn delete_user(
    app_state: web::Data<AppState>,
    info: web::Path<String>,
) -> impl Responder {
    let db = &app_state.db;

    // Mengonversi `i32` menjadi `i64` untuk digunakan dalam query
    let info_id = Thing::from(("m_user", Id::String(info.into_inner())));
    let is_delete = false;

    // Query untuk mencari user berdasarkan ID
    let update_result = db
    .query("UPDATE m_user SET is_delete = true WHERE id = $info_id AND is_delete = $is_delete")
    .bind(("info_id", info_id))
    .bind(("is_delete", is_delete))
    .await;

    match update_result {
        Ok(mut query_result) => {
            // let result: Vec<Value> = query_result.take(0).unwrap_or_default();
            let result: Vec<DataJsonUsers> = match query_result.take(0) {
                Ok(data) => {
                    println!("Data hasil delete: {:?}", data);
                    data
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal membaca hasil delete: {}", e),
                    );
                    return HttpResponse::InternalServerError().json(err);
                }
            };

            if result.is_empty() {
                // Tidak ada yang terupdate
                let err = ApiResponseError::new(
                    ApiErrorType::NotFound,
                    "User tidak ditemukan atau sudah dihapus".to_string(),
                );
                return HttpResponse::NotFound().json(err);
            }

            // Berhasil update
            let api_response = ApiResponse::new(
                200,
                "Sukses".to_string(),
                json!({ "user": "Data hasil delete" }),
            );
            HttpResponse::Ok().json(api_response)
        }
        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Database error: {}", e),
            );
            HttpResponse::InternalServerError().json(err)
        }
    }
}