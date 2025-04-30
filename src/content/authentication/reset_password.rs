use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::env;
use surrealdb::sql::{Thing, Id};

use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client; // Gunakan Client, bukan Ws

use anyhow::{Result, anyhow};

use crate::model_helpers::{CountPagination, ParamPagination};
use crate::model_reset_password::{DataJsonCurrentPass, DataPasswordChange};
use crate::model_user::{CountResult, DataJsonUsersAll};
use crate::{
    authentication::auth::{cek_user, generate_jwt_token}, 
    error_response::{ApiErrorType, ApiResponseError}, 
    model_auth::{Record, RequestVerifyEmail, UserAuth}, 
    model_reset_password::{AuthStatusReset, BodyJsonReset, DataJsonResetPass, DataPassword, FieldChangePasswrod, ResetPasswordBody, ResetPasswordCurrent}, 
    send_email::send_verification_email, 
    sukses_response::ApiResponse, 
    utilities::helpers::now_jakarta_time, 
    AppState
};

const TABLE_USER: &str = "m_user";

use bcrypt::verify;
extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash};

use super::auth::validate_jwt_token;
// use super::send_email::send_verification_email;

pub async fn request_cek_email(body: web::Json<RequestVerifyEmail>,app_state: web::Data<AppState>,) -> impl Responder {
    let db = app_state.db.clone();
    println!("email cek data {}", body.email);

    match cek_user(&db, body.email.clone()).await {
        Ok(Some(_user)) => {
            
            let formatted = now_jakarta_time();
            let ratio = 30;

            let verification_token = generate_jwt_token(&body.email, ratio).await;

            let data = BodyJsonReset {
                nama: _user.name.clone(),
                email: body.email.clone(),
                user: _user.id,// ✅ Perbaikan
                created_at: formatted.clone(),
                updated_at: formatted.clone(),
                token: verification_token.clone(),
                durasi: ratio
            };

            match create_request_reset(&db, data).await {
                Ok(_) => {
                    let payload = json!({
                        "email": body.email,
                        "token": verification_token.clone(),
                    });
                    
                    let api_response = ApiResponse::new(
                        200,
                        "Sukses request reset password".to_string(),
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

async fn create_request_reset(db: &Surreal<Client>, body: BodyJsonReset) -> Result<Option<Record>> {
    let email = body.email.clone();
    let nama = body.nama.clone();
    let token = body.token.clone();
    let durasi = body.durasi;
    let base_url = env::var("APP_BASE_URL_WEB").expect("APP_BASE_URL_WEB not set");

    // Cek apakah ada token aktif
    let existing_tokens: Vec<FieldChangePasswrod> = db
        .query("SELECT * FROM t_password_change WHERE email = $email AND token != '' LIMIT 1")
        .bind(("email", email.clone()))
        .await?
        .take(0)?;

    if let Some(existing) = existing_tokens.into_iter().next() {
        // Validasi token lama jika ada
        match validate_jwt_token(&existing.token).await {
            Ok(_) => {
                // Token lama masih valid → tolak permintaan baru
                return Err(anyhow!("Gagal request reset password, karena Anda sudah melakukan permintaan sebelumnya."));
            }
            Err(_) => {
                // Token lama sudah tidak valid → kosongkan
                db.query("UPDATE t_password_change SET token = '' WHERE email = $email")
                    .bind(("email", email.clone()))
                    .await?;
            }
        }
    }

    // Email HTML
    let title = "Request reset password".to_string();
    let bodyhtml = format!(
        "<p>Hai, <strong>{}</strong>,</p>
        <p>Silakan klik tautan berikut untuk reset password akun Anda:</p>
        <center>
            <a href='{}/reset-password?&token={}' style='background:#2978ff;border:15px solid #2978ff;padding:0 10px;color:#ffffff;font-size:15px;line-height:1;text-align:center;text-decoration:none;display:block;border-radius:10px' target='_blank'>
                Reset password
            </a>
        </center>
        <p>Tautan akan berakhir dalam {} menit</p>
        <p>Terimakasih</p>
        <p>Hormat Kami,</p>",
        nama, base_url, token, durasi
    );

    // Buat entri baru
    let created: Option<Record> = db
        .create("t_password_change")
        .content(body)
        .await?;

    if let Some(ref record) = created {
        if let Err(e) = send_verification_email(&email, &title, &bodyhtml).await {
            // ✅ CLONE dulu ID agar bisa digunakan setelah await
            let record_id = record.id.clone();
    
            // Rollback entri reset jika email gagal dikirim
            db.query("DELETE t_password_change WHERE id = $id")
                .bind(("id", record_id))
                .await?;
    
            return Err(anyhow!("Email verification failed: {}", e));
        }
    }

    Ok(created)
}

pub async fn request_reset_password(body: web::Json<ResetPasswordBody>,app_state: web::Data<AppState>,) -> Result<HttpResponse, actix_web::Error> {
    let db = app_state.db.clone();
    let verification_token = body.token.clone();

    match validate_jwt_token(&verification_token).await {
        Ok(_claims) => {
            match cek_token_change(&db, verification_token).await {
                Ok(Some(_user)) => {
                    
                    let formatted = now_jakarta_time();

                    let id_user = _user.user.to_string(); 
                    
                    let new_ = body.new_password.clone();
                    let old_ = body.old_password.clone();
                    let confirm_ = body.confirm_password.clone();

                    let hashed_password_old = hash(&old_, DEFAULT_COST)
                        .map_err(|e| actix_web::error::ErrorInternalServerError(
                            ApiResponseError::new(
                                ApiErrorType::InternalServerError, 
                                format!("Gagal hashing password: {}", e))
                        ))?;

                    let hashed_password_new = hash(&new_, DEFAULT_COST)
                        .map_err(|e| actix_web::error::ErrorInternalServerError(
                            ApiResponseError::new(
                                ApiErrorType::InternalServerError, 
                                format!("Gagal hashing password: {}", e))
                        ))?;

                    let data = DataJsonResetPass {
                        user_id: id_user,
                        updated_at: formatted,
                        old_password: old_,
                        new_password: new_,
                        confirm_password: confirm_,
                        hashed_old_password: hashed_password_old,
                        hashed_new_password: hashed_password_new,
                        email: _user.email,
                        token_verification: _user.token
                    };

                    match authenticate_user_reset(&db, data.clone()).await {
                        Ok(AuthStatusReset::Success(_user)) => {
                            
                            match update_m_user(&db, data).await {
                                Ok(data) => {
                                    let api_response = ApiResponse::new(
                                        200,
                                        "Password berhasil dirubah. Silakan login kembali dengan password baru Anda".to_string(),
                                        data,
                                    );
                                    Ok(HttpResponse::Ok().json(api_response))
                                }
                            
                                Err(e) => {
                                    println!("Kesalahan database: {:?}", e);
                                    let error_details = serde_json::to_string(&json!({ "error": format!("{:?}", e) }))
                                        .unwrap_or_default();
                                    let err = ApiResponseError::new(
                                        ApiErrorType::InternalServerError,
                                        error_details,
                                    );
                                    Ok(HttpResponse::InternalServerError().json(err))
                                }
                            }
                           
                        }

                        Ok(AuthStatusReset::NotVerified) => {
                            let res = ApiResponseError::new(
                                ApiErrorType::Unauthorized,
                                "Akun belum diverifikasi. Silakan cek email Anda.".to_string(),
                            );
                            Ok(HttpResponse::Unauthorized().json(res))
                        }

                        Ok(AuthStatusReset::NotFound) => {
                            let res = ApiResponseError::new(
                                ApiErrorType::NotFound,
                                "Data tidak ditemukan.".to_string(),
                            );
                            Ok(HttpResponse::NotFound().json(res))
                        }

                        Ok(AuthStatusReset::InvalidCredentials) => {
                            let res = ApiResponseError::new(
                                ApiErrorType::Unauthorized,
                                "Invalid Credentials.".to_string(),
                            );
                            Ok(HttpResponse::Unauthorized().json(res))
                        }

                        Ok(AuthStatusReset::PasswordNotMatch) => {
                            let res = ApiResponseError::new(
                                ApiErrorType::BadRequest,
                                "Konfirmasi password tidak cocok.".to_string(),
                            );
                            Ok(HttpResponse::BadRequest().json(res))
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

                Ok(None) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::NotFound,
                        "Token tidak ditemukan".to_string(),
                    );
                    Ok(HttpResponse::NotFound().json(err))
                }

                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Database error token: {}", e),
                    );
                    Ok(HttpResponse::InternalServerError().json(err))
                }
            }
        }

        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Token error: {}", e),
            );
            Ok(HttpResponse::InternalServerError().json(err))
        }
    }
}

async fn cek_token_change(db: &Surreal<Client>, verification_token: String) -> Result<Option<FieldChangePasswrod>, surrealdb::Error> {
    
    let users: Vec<FieldChangePasswrod> = db
        .query("SELECT * FROM t_password_change WHERE token = $token_value LIMIT 1")
        .bind(("token_value", verification_token))
        .await?
        .take(0)?;

    Ok(users.into_iter().next())
}

async fn authenticate_user_reset(db: &Surreal<Client>, body: DataJsonResetPass) -> Result<AuthStatusReset, surrealdb::Error> {
    let user_email = body.email.clone();

    println!("SELECT * FROM m_user WHERE email = {} AND is_delete = false LIMIT 1", user_email);
    let users: Vec<UserAuth> = db
        .query(&format!("SELECT * FROM m_user WHERE email = '{}' AND is_delete = false LIMIT 1", user_email))
        .await?
        .take(0)?;

    if let Some(user) = users.into_iter().next() {

        if body.new_password != body.confirm_password {
            return Ok(AuthStatusReset::PasswordNotMatch);
        }

        if !user.is_verified {
            return Ok(AuthStatusReset::NotVerified);
        }
        println!("hashed_old_password : {}", body.hashed_old_password);
        println!("user.password : {}", user.password);
        if verify(body.old_password, &user.password).unwrap_or(false) {
            Ok(AuthStatusReset::Success(user))
        } else {
            Ok(AuthStatusReset::InvalidCredentials)
        }


    } else {
        Ok(AuthStatusReset::NotFound)
    }
}

async fn update_m_user(db: &Surreal<Client>,body: DataJsonResetPass,) -> Result<DataPassword, surrealdb::Error> {
    let user_id = body.user_id.clone();
    let user_id_thing: Thing = user_id.parse().unwrap();

    // Update password dan updated_at
    let query = r#"
        BEGIN TRANSACTION;
        
        UPDATE $user_id SET password = $new_password, updated_at = $updated_at;
        
        UPDATE t_password_change 
        SET 
            new_password = $new_password, 
            old_password = $old_password, 
            password_change_timestamp = $timestamp, 
            token = ''
        WHERE token = $token_value;
        
        COMMIT TRANSACTION;
    "#;

    let response = db
        .query(query)
        .bind(("user_id", user_id_thing))
        .bind(("new_password", body.hashed_new_password.clone()))
        .bind(("old_password", body.hashed_old_password.clone()))
        .bind(("updated_at", body.updated_at.clone()))
        .bind(("timestamp", body.updated_at.clone()))
        .bind(("token_value", body.token_verification.clone()))
        .await;
    match response {
        Ok(_) => Ok(DataPassword {
            password: body.new_password.clone(),
        }),
        Err(e) => {
            println!("Transaction failed: {:?}", e);
            Err(e)
        }
    }
}

pub async fn current_reset_password(info: web::Path<String>,body: web::Json<ResetPasswordCurrent>,app_state: web::Data<AppState>,) -> Result<HttpResponse, actix_web::Error> {
    let db = app_state.db.clone();
    
    let formatted = now_jakarta_time();
    let user_id_str = info.into_inner();

    match cek_user_id(&db, user_id_str).await {
        Ok(Some(users)) => {
            if !users.is_verified {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    "Email belum verified".to_string(),
                );
                return Ok(HttpResponse::InternalServerError().json(err));
            }

            if body.new_password != body.confirm_password {
                let err = ApiResponseError::new(
                    ApiErrorType::AuthError,
                    "Konfirmasi password tidak cocok".to_string(),
                );
                return Ok(HttpResponse::Unauthorized().json(err));
            }

            let new_ = body.new_password.clone();
            let old_ = body.old_password.clone();
            let confirm_ = body.confirm_password.clone();
            
            let hashed_password_old = hash(&old_, DEFAULT_COST)
                .map_err(|e| actix_web::error::ErrorInternalServerError(
                    ApiResponseError::new(
                        ApiErrorType::InternalServerError, 
                        format!("Gagal hashing password: {}", e))
                ))?;

            let hashed_password_new = hash(&new_, DEFAULT_COST)
                .map_err(|e| actix_web::error::ErrorInternalServerError(
                    ApiResponseError::new(
                        ApiErrorType::InternalServerError, 
                        format!("Gagal hashing password: {}", e))
                ))?;
                println!("hashed_password_old => {}",hashed_password_old.clone());
                println!("hashed_password_new => {}",hashed_password_new.clone());
                println!("password => {}", users.password);
                println!("old => {}", old_);
            if verify(&old_, &users.password).unwrap_or(false) {
                let data = DataJsonCurrentPass {
                    user_id: users.id.to_string(),
                    updated_at: formatted,
                    old_password: old_,
                    new_password: new_,
                    confirm_password: confirm_,
                    hashed_old_password: hashed_password_old,
                    hashed_new_password: hashed_password_new,
                    email: users.email,
                };
    
                match update_password(&db, data).await {
                    Ok(data) => {
                        let api_response = ApiResponse::new(
                            200,
                            "Password berhasil dirubah. Silakan login kembali dengan password baru Anda".to_string(),
                            data,
                        );
                        Ok(HttpResponse::Ok().json(api_response))
                    }
                
                    Err(e) => {
                        println!("Kesalahan database: {:?}", e);
                        let error_details = serde_json::to_string(&json!({ "error": format!("{:?}", e) }))
                            .unwrap_or_default();
                        let err = ApiResponseError::new(
                            ApiErrorType::InternalServerError,
                            error_details,
                        );
                        Ok(HttpResponse::InternalServerError().json(err))
                    }
                }
            } else {
                let err = ApiResponseError::new(
                    ApiErrorType::AuthError,
                    "Invalid credential".to_string(),
                );
                return Ok(HttpResponse::Unauthorized().json(err));
            }
            
           
        }
        Ok(None) => {
            let err = ApiResponseError::new(
                ApiErrorType::NotFound,
                "Data tidak ditemukan".to_string(),
            );
            Ok(HttpResponse::NotFound().json(err))
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

async fn cek_user_id(
    db: &Surreal<Client>,
    id: String,
) -> Result<Option<DataJsonUsersAll>, surrealdb::Error> {
    let info_id = Thing::from((TABLE_USER, Id::String(id)));
    let is_delete = false;

    let mut result = db
        .query("SELECT * FROM m_user WHERE id = $info_id AND is_delete = $is_delete LIMIT 1")
        .bind(("info_id", info_id))
        .bind(("is_delete", is_delete))
        .await?;

    let users: Vec<DataJsonUsersAll> = result.take(0)?;

    Ok(users.into_iter().next())
}

async fn update_password(db: &Surreal<Client>,body: DataJsonCurrentPass,) -> Result<DataPassword, surrealdb::Error> {
    let user_id = body.user_id.clone();
    let user_id_thing: Thing = user_id.parse().unwrap();

    // Update password dan updated_at
    let query = r#"
        BEGIN TRANSACTION;
        
        UPDATE $user_id SET password = $new_password, updated_at = $updated_at;
        
       INSERT INTO t_password_change (
            user,
            new_password,
            old_password,
            password_change_timestamp,
            created_at,
            updated_at
        ) VALUES (
            $user_id,
            $new_password,
            $old_password,
            $timestamp,
            $created_at,
            $updated_at
        );
        
        COMMIT TRANSACTION;
    "#;

    let response = db
        .query(query)
        .bind(("user_id", user_id_thing))
        .bind(("new_password", body.hashed_new_password.clone()))
        .bind(("old_password", body.hashed_old_password.clone()))
        .bind(("updated_at", body.updated_at.clone()))
        .bind(("timestamp", body.updated_at.clone()))
        .bind(("created_at", body.updated_at.clone()))
        .bind(("updated_at", body.updated_at.clone()))
        .bind(("user_id_thing", user_id))
        .await;
    match response {
        Ok(_) => Ok(DataPassword {
            password: body.new_password.clone(),
        }),
        Err(e) => {
            println!("Transaction failed: {:?}", e);
            Err(e)
        }
    }
}

pub async fn get_password_change(
    info: web::Path<String>,
    app_state: web::Data<AppState>,
    query: web::Query<ParamPagination>,
) -> Result<HttpResponse, actix_web::Error> {
    let db = app_state.db.clone();
    let user_id_str = info.into_inner();

    let page = query.page;
    let limit =  query.limit;
    let offset = (query.page - 1) * query.limit;

    match change_password_history(&db, user_id_str.clone(), limit, offset).await {
        Ok(Some(users)) => {
            match get_count_password_change(&db, user_id_str.clone(), page, limit).await {
                Ok(pagination) => {
                    let data = json!({
                        "user": users,
                        "pagination": pagination
                    });

                    let api_response = ApiResponse::new(200, "success".to_string(), data);
                    Ok(HttpResponse::Ok().json(api_response))
                }
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Pagination error: {}", e),
                    );
                    Ok(HttpResponse::InternalServerError().json(err))
                }
            }
        }
        Ok(None) => {
            let err = ApiResponseError::new(
                ApiErrorType::NotFound,
                "Data tidak ditemukan".to_string(),
            );
            Ok(HttpResponse::NotFound().json(err))
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

async fn change_password_history(
    db: &Surreal<Client>,
    id: String, limit: i32, offset: i32
) -> Result<Option<DataPasswordChange>, surrealdb::Error> {
    let info_id = Thing::from((TABLE_USER, Id::String(id)));

    let mut result = db
        .query("SELECT * FROM t_password_change WHERE user = $info_id LIMIT $limit START $offset")
        .bind(("info_id", info_id))
        .bind(("limit", limit))
        .bind(("offset", offset))
        .await?;

    let users: Vec<DataPasswordChange> = result.take(0)?;

    Ok(users.into_iter().next())
}

async fn get_count_password_change(
    db: &Surreal<Client>,
    user_id: String, page: i32, limit: i32
) -> Result<CountPagination, surrealdb::Error> {
    let user_thing = Thing::from((TABLE_USER, Id::String(user_id)));

    // Hitung total record
    let mut count_result = db
        .query("RETURN { count: count((SELECT id FROM t_password_change WHERE user = $user_id)) }")
        .bind(("user_id", user_thing))
        .await?;

    let count_vec: Vec<CountResult> = count_result.take(0).unwrap_or_default();
    let total = count_vec.get(0).map(|c| c.count).unwrap_or(0);

    let total_pages = ((total as f64) / (limit as f64)).ceil() as i64;

    let pagination = CountPagination {
        total,
        page,
        limit,
        total_pages,
        count_per_page: total.min(limit as i64) as i32, // misalnya halaman pertama
    };

    Ok(pagination)
}
