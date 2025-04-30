use actix_web::{web, HttpResponse, Responder};
use reqwest::{redirect::Policy, Client, Result};

use crate::{
    authentication::{
        auth::{login, refresh_token, register, request_token_verify_email}, 
        email_verify::verify_email_handler, 
        reset_password::{current_reset_password, get_password_change, request_cek_email, request_reset_password}
    }, 
    error_response::{ApiErrorType, ApiResponseError}, 
    user_management::{
        create_user::create_user, 
        delete_user::delete_user, 
        export_data::export_users, 
        get_all_user::get_all_user, 
        get_user::{get_current_user, get_user}, 
        import::import_file_user, 
        status_user::update_status, 
        update_user::{update_current_user, update_user}
    }    
};

pub fn get_client() -> Result<Client>{
    reqwest::ClientBuilder::new()
            .cookie_store(true)
            .danger_accept_invalid_certs(true)
            .redirect(Policy::limited(20))
            .build()
}

pub async fn tes() -> impl Responder {
    HttpResponse::Ok().json("ok")
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/tes")  // ✅ Tambahkan ini
            .route(web::get().to(tes)),
    ).service(
        web::resource("/register-user")
            .route(web::post().to(register)),
    ).service(
            web::resource("/login")
            .route(web::post().to(login)),
    ).service(
        web::resource("/refresh_token")
            .route(web::post().to(refresh_token)),
    ).service(
        web::resource("/verify-email")
            .route(web::get().to(verify_email_handler)),
    ).service(
        web::resource("/refresh-verify-email")
            .route(web::post().to(request_token_verify_email)),
    ).service(
        web::resource("/request-reset-password")
            .route(web::post().to(request_cek_email)),
    ).service(
        web::resource("/reset-password")
            .route(web::post().to(request_reset_password)),
    ).service(
        web::resource("/users")
            .route(web::get().to(get_all_user)),
    ).service(
        web::resource("/get-user/{id}")
            .route(web::get().to(get_user)),
    ).service(
        web::resource("/create-user")
            .route(web::post().to(create_user)),
    ).service(
        web::resource("/delete-user/{id}")
            .route(web::get().to(delete_user)),
    ).service(
        web::resource("/update-user/{id}")
            .route(web::post().to(update_user)),
    ).service(
        web::resource("/current-user/{id}")
            .route(web::post().to(update_current_user)),
    ).service(
        web::resource("/get-current-user/{id}")
            .route(web::get().to(get_current_user)),
    ).service(
        web::resource("/change-password/{id}")
            .route(web::post().to(current_reset_password)),
    ).service(
        web::resource("/get-password-user/{id}")
            .route(web::get().to(get_password_change)),
    ).service(
        web::resource("/update-status/{id}")
            .route(web::post().to(update_status)),
    ).service(
        web::resource("/import/users")
            .route(web::post().to(import_file_user)),
    ).service(
        web::resource("/export/users")
            .route(web::get().to(export_users)),
    )
    .default_service(web::route().to(not_found)); 
}

async fn not_found() -> impl Responder {
    let api_response = ApiResponseError::new(
        ApiErrorType::NotFound, // Sesuaikan dengan tipe error NotFound
        "Route not found".to_string(),
    );
    
    HttpResponse::NotFound().json(api_response)
}