use actix_web::{web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use chrono_tz::Asia::Jakarta;
use serde_json::json;

use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client; // Gunakan Client, bukan Ws
use surrealdb::sql::{Thing, Id};
use bcrypt::verify;
use dotenv::dotenv;
use std::env;
// use rand::{distributions::Alphanumeric, Rng};
use jsonwebtoken::{decode, Validation, DecodingKey};
use jsonwebtoken::errors::ErrorKind;

use anyhow::{Result, anyhow};

use crate::{
    error_response::{ApiErrorType, ApiResponseError},
    send_email::send_verification_email,
    sukses_response::ApiResponse,
    model_auth::{AuthStatus, Claims, CreateToken, DataJsonReqVerifyEmail, NameEmail, Record, RefeshToken, 
        RequestVerifyEmail, ResponseData, ResponseUser, TokenVerifyEmail, UserAuth, UserLogin},
    model_auth::{BodyJson, DataJson}, 
    AppState,
    utilities::helpers::now_jakarta_time
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash};

use jsonwebtoken::{encode, Header, EncodingKey};

pub async fn register(body: web::Json<BodyJson>, app_state: web::Data<AppState>,) -> impl Responder {
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
            let formatted = now_jakarta_time();

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
        .query("SELECT * FROM m_user WHERE email = $email AND is_delete = false LIMIT 1")
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
                .query("DELETE m_user WHERE email = $email AND is_delete = false")
                .bind(("email", email))
                .await;
            return Err(anyhow!("Email verification failed: {}", e));
        }
    }

    Ok(created)
}

pub async fn generate_jwt_token(email: &str, duration: i32) -> String {
    dotenv().ok(); // Load .env file

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET not set in .env");
    let expiration_utc = Utc::now() + Duration::minutes(duration.into());
    let expiration_wib = expiration_utc.with_timezone(&Jakarta);

    println!("expiration : {}", expiration_wib);

    let claims = TokenVerifyEmail {
        email: email.to_owned(),
        exp: expiration_wib.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("Failed to encode JWT")
}

pub async fn login(body: web::Json<UserLogin>,app_state: web::Data<AppState>,) -> impl Responder {
    let db = app_state.db.clone();
    let email = body.username.clone();

    match authenticate_user(&db, email.clone(), body.password.clone()).await {
        Ok(AuthStatus::Success(user)) => {
            let email_clone = user.email.clone();
            let data = ResponseData {
                name: user.name,
                email: user.email,
                user_description: user.user_description,
                project_id: user.project_id,
                role_type: user.role_type,
            };

            let role_str = user.role_type;
            let token_jwt = generate_jwt(&email_clone, role_str);

            let response_data = ResponseUser {
                token: token_jwt,
                user: data,
            };

            let api_response =
                ApiResponse::new(200, "Login successful".to_string(), response_data);
            HttpResponse::Ok().json(api_response)
        }

        Ok(AuthStatus::NotVerified) => {
            let res = ApiResponseError::new(
                ApiErrorType::Unauthorized,
                "Akun belum diverifikasi. Silakan cek email Anda.".to_string(),
            );
            HttpResponse::Unauthorized().json(res)
        }

        Ok(AuthStatus::InvalidCredentials) => {
            let res = ApiResponseError::new(
                ApiErrorType::Unauthorized,
                "Email atau password salah.".to_string(),
            );
            HttpResponse::Unauthorized().json(res)
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

async fn authenticate_user(db: &Surreal<Client>,email: String, password: String,) -> Result<AuthStatus, surrealdb::Error> {
    let users: Vec<UserAuth> = db
        .query("SELECT * FROM m_user WHERE email = $email AND is_delete = false LIMIT 1")
        .bind(("email", email.clone()))
        .await?
        .take(0)?;

    if let Some(user) = users.into_iter().next() {
        if !user.is_verified {
            return Ok(AuthStatus::NotVerified);
        }

        if verify(&password, &user.password).unwrap_or(false) {
            Ok(AuthStatus::Success(user))
        } else {
            Ok(AuthStatus::InvalidCredentials)
        }
    } else {
        Ok(AuthStatus::InvalidCredentials)
    }
}

fn generate_jwt(user_id: &str, user_role: i32) -> String {
    dotenv().ok(); // Load .env file

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET not set in .env");
    let expiration_utc = Utc::now() + Duration::hours(1);
    let expiration_wib = expiration_utc.with_timezone(&Jakarta);

    let claims = Claims {
        sub: user_id.to_owned(),
        role: user_role,
        exp: expiration_wib.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ).expect("Failed to encode JWT")
}

pub async fn refresh_token(body: web::Json<RefeshToken>) -> impl Responder {
    let new_token = generate_jwt(&body.user_id, body.user_role);
    
    let data = CreateToken {
        token: new_token,
    };
    
    let api_response = ApiResponse::new(
        200, 
        "Token refreshed successfully".to_string(), 
        data);
    return HttpResponse::Ok().json(api_response);
    
}

pub async fn validate_jwt_token(token: &str) -> Result<TokenVerifyEmail, String> {
    dotenv().ok();
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET not set in .env");

    match decode::<TokenVerifyEmail>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(), // Validasi default (cek tanda tangan dan expiry)
    ) {
        Ok(token_data) => Ok(token_data.claims), // JWT valid, kembalikan claims
        Err(err) => match err.kind() {
            ErrorKind::ExpiredSignature => Err("Token has expired".to_string()),
            _ => Err("Invalid token".to_string()),
        },
    }
}

pub async fn request_token_verify_email(
    body: web::Json<RequestVerifyEmail>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let db = app_state.db.clone();
    println!("email cek data {}", body.email);

    match cek_user(&db, body.email.clone()).await {
        Ok(Some(_user)) => {
            // Ambil waktu sekarang di zona waktu Jakarta
            let formatted = now_jakarta_time();
            let ratio = 15;
            // Buat JWT token
            let verification_token = generate_jwt_token(&body.email, ratio).await;

            let data = DataJsonReqVerifyEmail {
                name: _user.name.clone(),
                email: body.email.clone(),
                updated_at: formatted.to_string(),
                verification_token: verification_token.clone(),
                durasi: ratio,
            };

            match create_verification_token(&db, data).await {
                Ok(_) => {
                    let payload = json!({
                        "name": _user.name,
                        "email": body.email,
                    });

                    let api_response = ApiResponse::new(
                        200,
                        "Token verifikasi berhasil dibuat. Silakan cek email Anda.".to_string(),
                        payload,
                    );

                    HttpResponse::Ok().json(api_response)
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal membuat token verifikasi: {}", e),
                    );
                    HttpResponse::InternalServerError().json(err)
                }
            }
        }
        Ok(None) => {
            let err = ApiResponseError::new(
                ApiErrorType::NotFound,
                "Email tidak ditemukan".to_string(),
            );
            HttpResponse::NotFound().json(err)
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

async fn create_verification_token(db: &Surreal<Client>, body: DataJsonReqVerifyEmail) -> Result<Option<Record>> {
    let email = body.email.clone();
    let verification_token = body.verification_token.clone();
    let updated_at = body.updated_at.clone();
    let durasi = body.durasi;
    let nama = body.name.clone(); // Pastikan field `name` tersedia di `DataJsonReqVerifyEmail`

    let base_url = env::var("APP_BASE_URL_WEB").expect("APP_BASE_URL_WEB not set");
   
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
                    verification_token,
                    durasi
                );

    // Jalankan UPDATE query
    let mut response = db
        .query("UPDATE m_user SET updated_at = $updated_at, verification_token = $verification_token WHERE email = $email AND is_delete = false RETURN AFTER")
        .bind(("updated_at", updated_at))
        .bind(("verification_token", verification_token.clone()))
        .bind(("email", email.clone()))
        .await?;

    // Ambil hasil record yang sudah di-update
    let created: Option<Record> = response.take(0)?;

    // Kalau berhasil update user
    if let Some(ref _user) = created {
        if let Err(e) =  send_verification_email(&email, &title, &bodyhtml).await {
            return Err(anyhow!("Email verification failed: {}", e));
        }
    }

    Ok(created)
}