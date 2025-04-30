use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use surrealdb::sql::{Thing, Id};

use crate::{
    error_response::{ApiErrorType, ApiResponseError}, 
    model_user::{BodyJsonProTy, DataJsonName, DataJsonUsers}, 
    sukses_response::ApiResponse, 
    utilities::helpers::now_jakarta_time,
    AppState
};

// const FIELD_IS_DELETE: &str = "is_delete";
const TABLE_USER: &str = "m_user";

pub async fn update_user(
    app_state: web::Data<AppState>,
    info: web::Path<String>,
    body: web::Json<BodyJsonProTy>
) -> impl Responder {
    let db = &app_state.db;

    let user_id_str = info.into_inner();
    let info_id = Thing::from((TABLE_USER, Id::String(user_id_str.clone())));
    let is_delete = false;

    let body_updated_at = now_jakarta_time();
    let body_project_id = body.project_id;
    let body_role_type = body.role_type;

    println!("info_id: {}", info_id);
    println!("nama table: {}", TABLE_USER);
    println!("body_updated_at: {}", body_updated_at);
    println!("body_project_id: {}", body_project_id);
    println!("body_role_type: {}", body_role_type);

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

    // Lanjut update user
    let update_result = db
        .query(
            r#"
            UPDATE m_user 
            SET updated_at = $body_updated_at, 
                project_id = $body_project_id, 
                role_type = $body_role_type 
            WHERE id = $info_id AND is_delete = $is_delete RETURN AFTER
        "#,
        )
        .bind(("body_updated_at", body_updated_at))
        .bind(("body_project_id", body_project_id))
        .bind(("body_role_type", body_role_type))
        .bind(("info_id", info_id))
        .bind(("is_delete", is_delete))
        .await;

    match update_result {
        Ok(mut query_result) => {
            let updated: Vec<DataJsonUsers> = match query_result.take(0) {
                Ok(data) => {
                    println!("Data hasil update: {:?}", data);
                    data
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal membaca hasil update: {}", e),
                    );
                    return HttpResponse::InternalServerError().json(err);
                }
            };
            if updated.is_empty() {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    "Update gagal dilakukan".to_string(),
                );
                return HttpResponse::InternalServerError().json(err);
            }

            let api_response = ApiResponse::new(
                200,
                "Sukses".to_string(),
                json!({ "user": "Update berhasil" }),
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

pub async fn update_current_user(
    app_state: web::Data<AppState>,
    info: web::Path<String>,
    body: web::Json<DataJsonName>
) -> impl Responder {
    let db = &app_state.db;

    let user_id_str = info.into_inner();
    let info_id = Thing::from((TABLE_USER, Id::String(user_id_str.clone())));
    let is_delete = false;

    let body_updated_at = now_jakarta_time();
    let name = body.name.clone();
    let user_description = body.user_description.clone();

    println!("info_id: {}", info_id);
    println!("nama table: {}", TABLE_USER);
    println!("name: {}", name);

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

    // Lanjut update user
    let update_result = db
        .query(
            r#"
            UPDATE m_user 
            SET updated_at = $body_updated_at, 
                name = $name,
                user_description = $user_description 
            WHERE id = $info_id AND is_delete = $is_delete RETURN AFTER
        "#,
        )
        .bind(("body_updated_at", body_updated_at))
        .bind(("name", name))
        .bind(("user_description", user_description))
        .bind(("info_id", info_id))
        .bind(("is_delete", is_delete))
        .await;

    match update_result {
        Ok(mut query_result) => {
            let updated_users: Vec<DataJsonUsers> = match query_result.take(0) {
                Ok(data) => {
                    println!("Data hasil update: {:?}", data);
                    data
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal membaca hasil update: {}", e),
                    );
                    return HttpResponse::InternalServerError().json(err);
                }
            };
    
            if let Some(_user) = updated_users.into_iter().next() {
                let api_response = ApiResponse::new(
                    200,
                    "Sukses".to_string(),
                    json!({ "user": "Update berhasil" }),
                );
                HttpResponse::Ok().json(api_response)
            } else {
                let err = ApiResponseError::new(
                    ApiErrorType::NotFound,
                    "User tidak ditemukan".to_string(),
                );
                HttpResponse::NotFound().json(err)
            }
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