use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use surrealdb::sql::{Thing, Id};

use crate::{
    error_response::{ApiErrorType, ApiResponseError}, 
    model_user::DataJsonUsers, 
    sukses_response::ApiResponse,
    AppState
};

const TABLE_USER: &str = "m_user";

pub async fn get_user(
    app_state: web::Data<AppState>,
    info: web::Path<String>,
) -> impl Responder {
    let db = &app_state.db;

    // Mengonversi `i32` menjadi `i64` untuk digunakan dalam query
    let info_id = Thing::from(("m_user", Id::String(info.into_inner())));

    // Query untuk mencari user berdasarkan ID
    let mut _result = db
        .query("SELECT * FROM m_user WHERE id = $info_id LIMIT 1")
        .bind(("info_id", info_id))
        .await;

    // Mengecek hasil query dan mengambil hasilnya
    let users: Vec<DataJsonUsers> = match _result {
        Ok(mut query_result) => query_result.take(0).unwrap_or_default(),  // Mengambil hasil query
        Err(err) => {
            println!("❌ Query error: {}", err);
            return HttpResponse::InternalServerError().body("Failed to query user data");
        }
    };

    // Mengecek apakah user ditemukan
    if users.is_empty() {
        let err = ApiResponseError::new(
            ApiErrorType::NotFound,
            format!("Data tidak ditemukan"),
        );
        
        return HttpResponse::NotFound().json(err);
    }

    // Jika user ditemukan, mengembalikan response JSON
    let user_data = json!({
        "user": users[0],  // Ambil user pertama dari hasil query
    });

    // Menyusun API response
    let api_response = ApiResponse::new(200, "User found".to_string(), user_data);
    HttpResponse::Ok().json(api_response)
}

pub async fn get_current_user(
    app_state: web::Data<AppState>,
    info: web::Path<String>,
) -> impl Responder {
    let db = &app_state.db;

    let user_id_str = info.into_inner();
    let info_id = Thing::from((TABLE_USER, Id::String(user_id_str.clone())));
    let is_delete = false;

    println!("info_id: {}", info_id);
    println!("nama table: {}", TABLE_USER);

    // Cek apakah user dengan ID tersebut ada dan belum dihapus
    let result = db
        .query("SELECT * FROM m_user WHERE id = $info_id AND is_delete = $is_delete LIMIT 1")
        .bind(("info_id", info_id.clone()))
        .bind(("is_delete", is_delete))
        .await;

    let users: Vec<DataJsonUsers> = match result {
        Ok(mut query_result) => match query_result.take(0) {
            Ok(data) => data,
            Err(_) => {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    "Gagal mengambil data user".to_string(),
                );
                return HttpResponse::InternalServerError().json(err);
            }
        },
        Err(_) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                "Gagal menjalankan query cek user".to_string(),
            );
            return HttpResponse::InternalServerError().json(err);
        }
    };

    // Kalau user tidak ditemukan
    if users.is_empty() {
        let err = ApiResponseError::new(
            ApiErrorType::NotFound,
            "User tidak ditemukan".to_string(),
        );
        return HttpResponse::NotFound().json(err);
    }

    // Kalau user ditemukan
    if let Some(user) = users.into_iter().next() {
        let api_response = ApiResponse::new(
            200,
            "Sukses".to_string(),
            json!({ "user": user, "permission": [] }),
        );
        HttpResponse::Ok().json(api_response)
    } else {
        let err = ApiResponseError::new(
            ApiErrorType::InternalServerError,
            "User ditemukan tapi gagal dibaca".to_string(),
        );
        HttpResponse::InternalServerError().json(err)
    }
}
