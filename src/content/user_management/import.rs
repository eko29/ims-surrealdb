extern crate csv;

use actix_web::{web, HttpResponse};
use actix_multipart::Multipart;
use futures_util::StreamExt;
use uuid::Uuid;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use surrealdb::Surreal;
use calamine::{Reader as CalamineReader, open_workbook, RangeDeserializerBuilder, Xlsx};

use anyhow::{anyhow, Result};

use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

use crate::{
    model_auth::Record,
    model_export_import::{BodyFormJson, ImportFailedRecord, ImportResponse},
    send_email::send_verification_email,
    sukses_response::ApiResponse,
    authentication::auth::{generate_jwt_token, cek_user},
    model_auth::DataJson,
    utilities::helpers::now_jakarta_time,
    AppState,
    error_response::{ApiErrorType, ApiResponseError}
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash};

use surrealdb::engine::remote::ws::Client; // Gunakan Client, bukan Ws
use surrealdb::sql::{Thing, Id};
use std::env;

use std::io::Write;


pub async fn import_file_user(
    mut payload: Multipart,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let db = app_state.db.clone();

    let mut send_verification_email = false;
    let mut _update_existing = false;
    let mut uploaded_file_path = None;

    while let Some(field_result) = payload.next().await {
        let mut field = match field_result {
            Ok(f) => f,
            Err(e) => {
                // error!("Multipart error: {}", e);
                let err = ApiResponseError::new(
                    ApiErrorType::BadRequest,
                    format!("Jenis file tidak didukung: {}", e),
                );
                return Ok(HttpResponse::BadRequest().json(err));
            }
        };

        let content_disposition = field.content_disposition().cloned();
        let field_name = content_disposition
            .as_ref()
            .and_then(|cd| cd.get_name());

        match field_name {
            Some("file") => {
                let filename = content_disposition
                    .as_ref()
                    .and_then(|cd| cd.get_filename())
                    .unwrap_or("uploadfile");

                let ext = Path::new(filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                let temp_path = format!("./temp/{}_{}", Uuid::new_v4(), filename);
                let mut f = match File::create(&temp_path) {
                    Ok(file) => file,
                    Err(e) => {
                        let err = ApiResponseError::new(
                            ApiErrorType::BadRequest,
                            format!("Gagal membuat file: {}", e),
                        );
                        return Ok(HttpResponse::BadRequest().json(err));
                    }
                };
                
                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(d) => d,
                        Err(e) => {
                            let err = ApiResponseError::new(
                                ApiErrorType::BadRequest,
                                format!("Gagal membaca data file: {}", e),
                            );
                            return Ok(HttpResponse::BadRequest().json(err));
                        }
                    };
                
                    if let Err(e) = f.write_all(&data) {
                        let err = ApiResponseError::new(
                            ApiErrorType::InternalServerError,
                            format!("Gagal menulis data ke file: {}", e),
                        );
                        return Ok(HttpResponse::InternalServerError().json(err));
                    }
                }

                uploaded_file_path = Some((temp_path, ext));
            }

            Some("send_verification_email") => {
                let mut value = Vec::new(); // Pastikan value dideklarasikan
                while let Some(chunk) = field.next().await {
                    match chunk {
                        Ok(c) => value.extend_from_slice(&c),
                        Err(e) => {
                            let err = ApiResponseError::new(
                                ApiErrorType::BadRequest,
                                format!("Gagal membaca field send_verification_email: {}", e),
                            );
                            return Ok(HttpResponse::BadRequest().json(err));
                        }
                    }
                }
            
                let string_val = String::from_utf8_lossy(&value).to_string();
                send_verification_email = string_val.trim().to_lowercase() == "true";
            }
            
            Some("update_existing") => {
                let mut _update_existing = false; // Ubah nama variabel menjadi _update_existing
                while let Some(chunk) = field.next().await {
                    let chunk = match chunk {
                        Ok(c) => c,
                        Err(e) => {
                            let err = ApiResponseError::new(
                                ApiErrorType::BadRequest,
                                format!("Gagal membaca field update_existing: {}", e),
                            );
                            return Ok(HttpResponse::BadRequest().json(err));
                        }
                    };
                    _update_existing = chunk.to_vec() == b"true";
                }
            }

            _ => {}
        }
    }

    // Pastikan file sudah diterima
    let (path, _ext) = match uploaded_file_path {
        Some(v) => v,
        None => {
            let err = ApiResponseError::new(
                ApiErrorType::BadRequest,
                format!("File tidak ditemukan dalam form"),
            );
            return Ok(HttpResponse::BadRequest().json(err));
        }
    };

    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let users = match ext.as_str() {
        "txt" => read_txt_file(&path)
            .into_iter()
            .map(|(name, email, user_description, contractor, project_id, role_type)| 
                BodyFormJson { name, email, user_description, contractor, project_id, role_type })
            .collect(),
        "csv" => read_csv_file(&path),
        "xlsx" => read_excel_file(&path),
        _ => {
            eprintln!("Jenis file tidak didukung");
            let err = ApiResponseError::new(
                ApiErrorType::BadRequest,
                "Jenis file tidak didukung".to_string(),
            );
            return Ok(HttpResponse::BadRequest().json(err));
        }
    };

    let total_records = users.len();
    let mut created = 0;
    let updated = 0;
    let mut failed = 0;
    let mut failed_records = Vec::new();

    for (index, user) in users.into_iter().enumerate() {
        match cek_user(&db, user.email.clone()).await {
            Ok(Some(_user)) => {
                failed += 1;
                        failed_records.push(ImportFailedRecord {
                            row: index + 1,
                            email: user.email.clone(),
                            error: "User already exists".to_string(),
                        });
            }
            Ok(None) => {
                let formatted = now_jakarta_time();
                let ratio = 15;
                let verification_token = generate_jwt_token(&user.email, ratio).await;
                let pass = generate_random_password(12);
                
                let hashed_password = match hash(pass.clone(), DEFAULT_COST) {
                    Ok(hp) => hp,
                    Err(e) => {
                        let err = ApiResponseError::new(
                            ApiErrorType::InternalServerError,
                            format!("Gagal hashing password: {}", e),
                        );
                        return Ok(HttpResponse::InternalServerError().json(err))
                    }
                };

                let data = DataJson {
                    name: user.name,
                    email: user.email.clone(),
                    password: hashed_password,
                    user_description: user.user_description.clone(),
                    contractor: Thing::from(("m_contractor", Id::Number(user.contractor.into()))),
                    created_at: formatted.clone(),
                    updated_at: formatted.clone(),
                    project_id: user.project_id,
                    role_type: user.role_type,
                    is_verified: false,
                    verification_token: verification_token.clone(),
                    durasi: ratio,
                    is_delete: false,
                };
                
                match create_user(&db, data, send_verification_email, pass.clone()).await {
                    Ok(Some(_record)) => {
                        created += 1;
                    }
                    Ok(None) => {
                        failed += 1;
                        failed_records.push(ImportFailedRecord {
                            row: index + 1,
                            email: user.email.clone(),
                            error: "Unknown creation error".to_string(),
                        });
                    }
                    Err(e) => {
                        failed += 1;
                        failed_records.push(ImportFailedRecord {
                            row: index + 1,
                            email: user.email.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
            Err(e) => {
                failed += 1;
                failed_records.push(ImportFailedRecord {
                    row: index + 1,
                    email: user.email.clone(),
                    error: e.to_string(),
                });
            }
        }
        
    }

    let _ = std::fs::remove_file(&path);
    let response = ImportResponse {
        total_records,
        created,
        updated,
        failed,
        failed_records,
    };
    let pesan = format!("Import completed with {} users created and {} failures. See details for errors.", total_records, failed);
    let api_response = ApiResponse::new(
        200, 
        pesan, 
        response);
        
    Ok(HttpResponse::Ok().json(api_response).into())
}

fn read_txt_file(path: &str) -> Vec<(String, String, String, i32, i32, i32)> {
    let file = File::open(path).expect("Failed to open file");
    let reader = BufReader::new(file);

    reader.lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            if parts.len() == 6 {
                let contractor = parts[3].parse::<i32>().ok()?;
                let project_id = parts[4].parse::<i32>().ok()?;
                let role_type = parts[5].parse::<i32>().ok()?;

                Some((
                    parts[0].to_string(), // name
                    parts[1].to_string(), // email
                    parts[2].to_string(), // user_description
                    contractor,
                    project_id,
                    role_type,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn read_csv_file(path: &str) -> Vec<BodyFormJson> {
    let mut rdr = csv::Reader::from_path(path).expect("Failed to open CSV file");

    rdr.deserialize()
        .filter_map(Result::ok)
        .collect()
}

fn read_excel_file(path: &str) -> Vec<BodyFormJson> {
    let mut workbook: Xlsx<_> = open_workbook(path).expect("Cannot open Excel file");
    let range = workbook.worksheet_range("Sheet1")
        .expect("Cannot find sheet")
        .expect("Invalid range");

    let iter = RangeDeserializerBuilder::new().from_range(&range).unwrap();
    let mut users = Vec::new();
    for result in iter {
        if let Ok(user) = result {
            users.push(user);
        }
    }
    users
}

fn generate_random_password(length: usize) -> String {
    let mut rng = thread_rng();
    
    // Membuat password dengan kombinasi huruf dan angka
    let password: String = (0..length)
        .map(|_| rng.sample(Alphanumeric))  // Memilih secara acak karakter dari Alphanumeric
        .map(char::from)                    // Mengonversi ke char
        .collect();

    password
}

async fn create_user(db: &Surreal<Client>, body: DataJson, send_email: bool, password: String) -> Result<Option<Record>> {
    let base_url = env::var("APP_BASE_URL_WEB").expect("APP_BASE_URL_WEB not set");
    let email = body.email.clone();
    let token = body.verification_token.clone();
    let durasi = body.durasi;
    let nama = body.name.clone();
    let title = "Verifikasi Email Akun Anda".to_string();
    let bodyhtml = format!(
                    "<p>Hai, <strong>{}</strong>,</p>
                    <p>Selamat akun ada sudah terdaftar dengan:</p>
                    <div style='padding-left:30px;'>
                        <table style='border-collapse: collapse;'>
                            <tr>
                                <td><strong>Email</strong></td>
                                <td>:</td>
                                <td>{}</td>
                            </tr>
                            <tr>
                                <td><strong>Password</strong></td>
                                <td>:</td>
                                <td>{}</td>
                            </tr>
                        </table>
                    </div>
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
                    email,
                    password,
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
        if send_email == true{
            if let Err(e) = send_verification_email(&email, &title, &bodyhtml).await {
                // Email gagal → rollback manual
                let _ = db
                    .query("DELETE m_user WHERE email = $email")
                    .bind(("email", email))
                    .await;
                return Err(anyhow!("Email verification failed: {}", e));
            }
        }
    }

    Ok(created)
}


