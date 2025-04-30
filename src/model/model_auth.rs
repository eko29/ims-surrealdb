use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing; // ✅ Gunakan Thing untuk ID
// use chrono::NaiveDateTime;

#[derive(Debug)]
pub enum AuthStatus {
    Success(UserAuth),
    InvalidCredentials,
    NotVerified,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UserLogin {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct UserAuth {
    pub email: String,
    pub password: String,
    pub name: String,
    pub user_description: String,
    // pub contractor: i64,
    pub project_id: i32,
    pub role_type: i32,
    pub is_verified: bool,
    
}

#[derive(Serialize, Deserialize)]
pub struct ResponseData {
    pub name: String,
    pub email: String,
    pub user_description: String,
    // pub contractor: i64,
    pub project_id: i32,
    pub role_type: i32,
}

#[derive(Serialize, Deserialize)]
pub struct BodyJson {
    pub name: String,
    pub email: String,
    pub password: String,
    pub user_description: String,
    pub contractor: i32,
    pub project_id: i32,
    pub role_type: i32,
}

#[derive(Serialize, Deserialize)]
pub struct RequestVerifyEmail {
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJson {
    pub name: String,
    pub email: String,
    pub password: String,
    pub user_description: String,
    pub contractor: Thing,
    pub created_at: String,
    pub updated_at: String,
    pub project_id: i32,
    pub role_type: i32,
    pub is_verified: bool, // Default: false
    pub verification_token: String,
    pub durasi: i32,
    pub is_delete: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJsonReqVerifyEmail {
    pub name: String,
    pub email: String,
    pub updated_at: String,
    pub verification_token: String,
    pub durasi: i32
}
#[derive(Serialize, Deserialize)]
pub struct ChangePassword {
    pub username: String,
    pub old_password: String,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    pub id: Thing, // ✅ Menangani ID secara otomatis
}

#[derive(Debug, Deserialize)]
pub struct ContractorRecord {
    pub contractor_id: Thing, // ✅ Menangani ID secara otomatis
}

#[derive(Deserialize)]
pub struct NameEmail {
    pub id: Thing,
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: i32,
    pub exp: usize,
}

#[derive(Serialize, Deserialize)]
pub struct TokenVerifyEmail {
    pub email: String,
    pub exp: usize,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseUser<T> {
    pub token: String,
    pub user: T, // Make the data field public
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefeshToken {
    pub user_id: String, 
    pub user_role: i32
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateToken {
    pub token: String, 
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    // pub email: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyEmail {
    pub id: Thing,
    pub email: String,
    pub is_verified: bool, // Default: false
    pub verification_token: String,
}