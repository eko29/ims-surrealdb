
use actix_cors::Cors;
use actix_web::error::InternalError;
use actix_web::{
    http::header, 
    web, 
    App, 
    HttpResponse, 
    HttpServer
};
use env_logger::{self};
use dotenv::dotenv;
use ims::error_response::{ApiErrorType, ApiResponseError};
use ims::path::config;
use ims::{AppState, AuthMiddleware};

use std::env;
use surrealdb::{opt::auth::Root, Surreal};

extern crate serde_derive;

use std::sync::{Arc, LazyLock};
use std::time::Duration;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::Config;

use log::{error, info};

static DB: LazyLock<Surreal<Client>> = LazyLock::new(Surreal::init);

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();
    info!("🚀 Server sedang dimulai...");

    // Ambil variabel lingkungan
    let port = match env::var("SERVER_PORT") {
        Ok(p) => p.parse::<u16>().unwrap_or(9001),
        Err(_) => {
            error!("⚠️ SERVER_PORT tidak ditemukan, menggunakan default 9001");
            9001
        }
    };

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| ApiErrorType::EnvVarError)
        .unwrap_or_else(|_e| {
            error!("⚠️ DATABASE_URL tidak ditemukan, menggunakan default localhost:8001");
            "localhost:8001".to_string()
        });

    let namespace = env::var("NAMESPACE").unwrap_or_else(|_| "ims_namespace".to_string());
    let database = env::var("DATABASE").unwrap_or_else(|_| "ims_database".to_string());

    println!("🛠 Konfigurasi:");
    println!("   🔹 SERVER_PORT: {}", port);
    println!("   🔹 DATABASE_URL: {}", database_url);
    println!("   🔹 NAMESPACE: {}", namespace);
    println!("   🔹 DATABASE: {}", database);

    let config_time = Config::default().query_timeout(Duration::from_millis(1500));
    let db = DB.clone();
    // Coba koneksi ke SurrealDB
    match db.connect::<Ws>((&database_url, config_time)).await {
        Ok(_) => info!("✅ Berhasil terhubung ke SurrealDB."),
        Err(e) => {
            error!("❌ Gagal konek ke database: {}", e);
            return Ok(());
        }
    }

    // Autentikasi ke SurrealDB
    match db.signin(Root {
        username: "root",
        password: "root",
    }).await {
        Ok(_) => info!("✅ Autentikasi SurrealDB berhasil."),
        Err(e) => {
            error!("❌ Gagal autentikasi: {}", e);
            return Ok(());
        }
    }

    // Pilih namespace & database
    match db.use_ns(&namespace).use_db(&database).await {
        Ok(_) => info!("✅ Namespace '{}' dan database '{}' berhasil dipilih.", namespace, database),
        Err(e) => {
            error!("❌ Gagal memilih database: {}", e);
            return Ok(());
        }
    }

    if db.health().await.is_err() {
        error!("❌ Database tidak siap! Pastikan SurrealDB berjalan.");
        return Ok(());
    }

    let db = Arc::new(DB.clone());
    let app_state = web::Data::new(AppState { db: db.clone() }); // 🔥 Bungkus dalam `AppState`

    println!("lanjut jsconfig");
    let json_cfg = web::JsonConfig::default()
        .limit(104857600)
        .error_handler(|err, _req| {
            error!("❌ JSON parsing error: {:?}", err);

            let api_response = ApiResponseError::new(
                ApiErrorType::NotFound,  
                format!("{}", err),
            );

            let response = HttpResponse::BadRequest().json(api_response);
            InternalError::from_response(err, response).into()
        });
    println!("jsconfig akhir");
    // Menjalankan server HTTP
    info!("🚀 Server berjalan di http://localhost:{}", port);
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600);
        
        App::new()
            .wrap(cors)
            // .wrap(AuthMiddleware { db_pool: Arc::as_ref(&db).clone() }) // 🔥 Gunakan db.clone()
            .wrap(AuthMiddleware::new(db.clone()))
            .app_data(app_state.clone()) // 🔥 Masukkan ke dalam Actix-web
            .app_data(json_cfg.clone())
            .configure(config)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}