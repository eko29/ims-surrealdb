use actix_web::{web, HttpResponse, Responder};
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client; // Gunakan Client, bukan Ws

use crate::{
    authentication::auth::validate_jwt_token, error_response::{ApiErrorType, ApiResponseError}, model_auth::{VerifyEmail, VerifyRequest}, sukses_response::ApiResponse, utilities::helpers::now_jakarta_time, AppState
};

use serde_json::json;

pub async fn verify_email_handler(
    app_state: web::Data<AppState>,
    query: web::Query<VerifyRequest>,
) -> impl Responder {
    let token = query.token.clone();
    // let email = query.email.clone();
    let db = app_state.db.clone();

    println!("Coba verifikasi token: {}", token);
    // Kita perlu cari user berdasarkan token saja (bukan email, karena user klik dari link)
    match verify_token(&db, token).await {
        Ok(Some(user)) => {
            let api_response = ApiResponse::new(
                200,
                "Email verified successfully".to_string(),
                json!({
                    "email": user.email,
                    "is_verified": user.is_verified,
                }),
            );
            HttpResponse::Ok().json(api_response)
        }
        Ok(None) => {
            let err = ApiResponseError::new(
                ApiErrorType::Unauthorized,
                "Invalid or expired verification token".to_string(),
            );
            HttpResponse::Unauthorized().json(err)
        }
        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Database error: {:?}", e),
            );
            HttpResponse::InternalServerError().json(err)
        }
    }
}

async fn verify_token(
    db: &Surreal<Client>,
    // email: String,
    verification_token: String,
) -> Result<Option<VerifyEmail>, surrealdb::Error> {
    println!("SELECT * FROM m_user WHERE verification_token = {} AND is_delete = false LIMIT 1", verification_token);
    println!("Cek token: {}", verification_token);

    match validate_jwt_token(&verification_token).await {
        Ok(claims) => {
            println!("✅ JWT Valid! email: {} | exp: {}", claims.email, claims.exp);
            let mut users: Vec<VerifyEmail> = db
                .query("SELECT * FROM m_user WHERE verification_token = $verification_token AND is_delete = false LIMIT 1")
                .bind(("verification_token", verification_token.clone()))
                .await?
                .take(0)?;

            if let Some(mut user) = users.pop() {
                println!("Token valid untuk user: {}", user.email);
                
                let formatted = now_jakarta_time();
                // Update status user
                user.is_verified = true;
                user.verification_token = "".to_string();
                let user_id = user.id.clone();

                db.query("UPDATE $user_id SET is_verified = true, email_veriied_at = $timestamp, verification_token = ''")
                    .bind(("user_id", user_id))            
                    .bind(("timestamp", formatted))
                    .await?;

                Ok(Some(user))
            } else {
                println!("Token tidak ditemukan");
                Ok(None)
            }
        }
        Err(err) => {
            println!("❌ JWT Error: {}", err);
            return Ok(None); // atau bisa pakai Err(anyhow!(...)) kalau ingin bubble up error
        }
    }
    
}