use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use chrono_tz::Asia::Jakarta;
use serde_json::json;

use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client; // Gunakan Client, bukan Ws
use surrealdb::sql::{Thing, Id};
use std::env;

use anyhow::{Result, anyhow};

use crate::{
    authentication::auth::generate_jwt_token,
    error_response::{ApiErrorType, ApiResponseError},
    send_email::send_verification_email,
    sukses_response::ApiResponse,
    model_auth::{NameEmail, Record},
    model_auth::{BodyJson, DataJson}, 
    AppState
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash};

pub async fn create_user(body: web::Json<BodyJson>, app_state: web::Data<AppState>,) -> impl Responder {
    let db = app_state.db.clone();
    // let body_email = body.email.clone(); // ✅ Konversi ke &str
    println!("email cek data {}", body.email.clone());
    match cek_user(&db, body.email.clone()).await {
        Ok(Some(_user)) => {
            // Jika user sudah ada, kembalikan response "User already exists"
            let err = ApiResponseError::new(
                ApiErrorType::Conflict,
                format!("User already exists"),
            );
            return HttpResponse::Conflict().json(err);
        }
        Ok(None) => {
            let now_utc = Utc::now();
            let jakarta_time = now_utc.with_timezone(&Jakarta);
            let formatted = jakarta_time.format("%Y-%m-%d %H:%M:%S").to_string();

            let password = &body.password;

            // Hash password dengan bcrypt
            let hashed_password = match hash(password, DEFAULT_COST) {
                Ok(hp) => hp,
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal hashing password: {}", e),
                    );
                    return HttpResponse::InternalServerError().json(err)
                }
            };
            let ratio = 15;
            let verification_token = generate_jwt_token(&body.email, ratio).await;
            
            let data = DataJson {
                name: body.name.clone(),
                email: body.email.clone(),
                password: hashed_password,
                user_description: body.user_description.clone(), // Clone untuk String
                contractor: Thing::from(("m_contractor", Id::Number(body.contractor.into()))),// ✅ Perbaikan
                created_at: formatted.clone(),
                updated_at: formatted.clone(),
                project_id: body.project_id, // Dereference untuk i32
                role_type: body.role_type, // 🔹 Pastikan nilai ini diberikan
                is_verified: false,
                verification_token: verification_token.clone(),
                durasi: ratio,
                is_delete: false
            };

            match create_authenticate(&db, data).await {
                Ok(_) => {
                    let payload = json!({
                        "name": body.name,
                        "password": password,
                        "email": body.email,
                        "token": verification_token.clone(),
                    });

                   
                    let api_response = ApiResponse::new(
                        200, 
                        "User registered successfully, Silakan cek email untuk melakukan verifikasi email".to_string(), 
                        payload);
                    return HttpResponse::Ok().json(api_response);

                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Error disini: {}", e),
                    );
                    return HttpResponse::InternalServerError().json(err)
                }
            }
        
        }
        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Database error kenapa: {}", e),
            );
            HttpResponse::InternalServerError().json(err) // Mengembalikan HttpResponse
        }
    }
}

pub async fn cek_user(db: &Surreal<Client>, email: String) -> Result<Option<NameEmail>, surrealdb::Error> {
    let users: Vec<NameEmail> = db
        .query("SELECT * FROM m_user WHERE email = $email LIMIT 1")
        .bind(("email", email))
        .await?
        .take(0)?;

    Ok(users.into_iter().next()) // Ambil elemen pertama sebagai Option<BodyJson>
}

async fn create_authenticate(db: &Surreal<Client>, body: DataJson) -> Result<Option<Record>> {
    let base_url = env::var("APP_BASE_URL_WEB").expect("APP_BASE_URL_WEB not set");
    let email = body.email.clone();
    let token = body.verification_token.clone();
    let durasi = body.durasi;
    let nama = body.name.clone();
    let title = "Verifikasi Email Akun Anda".to_string();
    let bodyhtml = format!(
                    "<p>Hai, <strong>{}</strong>,</p>
                    <p>Silakan klik tombol dibawah ini untuk melakukan verifikasi email:</p>
                    <center>
                        <a href='{}/verify-email?&token={}' style='background:#2978ff;border:15px solid #2978ff;padding:0 10px;color:#ffffff;font-size:15px;line-height:1;text-align:center;text-decoration:none;display:block;border-radius:10px' target='_blank'>
                            Verify email
                        </a>
                    </center>
                    <p>Tautan akan berakhir dalam {} menit</p>
                    <p>Terimakasih</p>
                    <p>Hormat Kami,</p>",
                    nama,
                    base_url,
                    token,
                    durasi
                );
    

    let created: Option<Record> = db
        .create("m_user")
        .content(body)
        .await?;

    // Kalau berhasil buat user
    if let Some(ref _record) = created {
        if let Err(e) = send_verification_email(&email, &title, &bodyhtml).await {
        // send_verification_email(&email, &token, &nama).await {
            // Email gagal → rollback manual
            let _ = db
                .query("DELETE m_user WHERE email = $email")
                .bind(("email", email))
                .await;
            return Err(anyhow!("Email verification failed: {}", e));
        }
    }

    Ok(created)
}