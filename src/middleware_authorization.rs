use surrealdb::{
    Surreal, 
    engine::remote::ws::Client
}; // ✅ Import SurrealDB

use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse, body::{BoxBody, EitherBody},
};
use actix_service::{Service, Transform};
use std::sync::Arc;
use std::future::{ready, Ready}; // ✅ Pastikan ready & Ready di-import
use futures_util::future::LocalBoxFuture; 
use futures_util::FutureExt;
use actix_web::http::header::AUTHORIZATION;
use dotenv::dotenv;
use std::env;
use jsonwebtoken::{decode, Validation, DecodingKey};
use jsonwebtoken::errors::ErrorKind;
use log::info;

use crate::{error_response::{ApiErrorType, ApiResponseError}, model_auth::Claims};

pub struct AuthMiddleware {
    // pub db_pool: Surreal<Client>,  // ✅ Ubah dari Surreal<Db> ke Surreal<Client>
    pub db_pool: Arc<Surreal<Client>>
}

impl AuthMiddleware {
    // pub fn new(db_pool: Surreal<Client>) -> Self {
    //     Self { db_pool }
    // }
    pub fn new(db_pool: Arc<Surreal<Client>>) -> Self {
        Self { db_pool }
    }
}

impl<S> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<EitherBody<BoxBody>>, Error = Error>
        + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<EitherBody<BoxBody>>;
    type Error = Error;
    type Transform = AuthMiddlewareMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareMiddleware {
            service: Arc::new(service),
            // db_pool: self.db_pool.clone(),
        }))
    }
}

pub struct AuthMiddlewareMiddleware<S> {
    service: Arc<S>,
    // db_pool: Surreal<Client>,
}

impl<S> Service<ServiceRequest> for AuthMiddlewareMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<EitherBody<BoxBody>>, Error = Error>
        + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<EitherBody<BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        // let db_pool = self.db_pool.clone();
        let path = req.path().to_string();

        async move {
            // Bypass authentication untuk endpoint login/register
            println!("link parame {}", path);
            
            let allowed_paths = ["/login", "/register-user", "/verify-email", "/refresh-verify-email", "/request-reset-password", "/reset-password"];

            if allowed_paths.contains(&path.as_str()) {
                return service.call(req).await;
            }

            // Ambil Authorization header
            let auth_header = req.headers().get(AUTHORIZATION);
            println!("Authorization auth_header: {:?}", auth_header);
            if auth_header.is_none() {
                let api_response = ApiResponseError::new(
                    ApiErrorType::Unauthorized, // Menggunakan ApiErrorType::Unauthorized
                    "No Authorization header present.".to_string(),
                );
                return unauthorized_response(req, api_response);
            }

            // let auth_str = auth_header.unwrap().to_str().unwrap_or_default();
            let auth_str = match auth_header.unwrap().to_str() {
                Ok(val) => val,
                Err(_) => {
                    let api_response = ApiResponseError::new(
                        ApiErrorType::Unauthorized,
                        "Invalid characters in Authorization header.".to_string(),
                    );
                    return unauthorized_response(req, api_response);
                }
            };

            let token = auth_str.trim_start_matches("Bearer ");

            match validate_jwt(token) {
                Ok(claims) => {
                    info!("JWT Valid! User: {} | Role: {}", claims.sub, claims.role);
                    return service.call(req).await;
                }
                Err(err) => {
                    let api_response = ApiResponseError::new(
                        ApiErrorType::Unauthorized,
                        err.to_string(),
                    );
                    unauthorized_response(req, api_response)
                }
            }
        }
        .boxed_local()
    }
}


// Fungsi untuk membuat response unauthorized
fn unauthorized_response(
    req: ServiceRequest,
    api_response: ApiResponseError,
) -> Result<ServiceResponse<EitherBody<BoxBody>>, Error> {
    let (req, _pl) = req.into_parts(); // Separating request from payload
    let res = HttpResponse::Unauthorized() // Set Unauthorized status
        .json(api_response) // Convert ApiResponseError to JSON
        .map_into_right_body(); // Convert response to appropriate body format for ServiceResponse

    Ok(ServiceResponse::new(req, res)) // Return the ServiceResponse with the request and response
}

fn validate_jwt(token: &str) -> Result<Claims, String> {
    dotenv().ok();
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET not set in .env");

    match decode::<Claims>(
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