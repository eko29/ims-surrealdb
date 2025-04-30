use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use super::model_auth::UserAuth;

#[derive(Debug)]
pub enum AuthStatusReset {
    Success(UserAuth),
    InvalidCredentials,
    NotVerified,
    PasswordNotMatch,
    NotFound
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BodyJsonReset {
    pub nama: String,
    pub email: String,
    pub user: Thing,
    pub created_at: String,
    pub updated_at: String,
    pub token: String,
    pub durasi: i32
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResetPasswordBody {
    pub token: String,
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ResetPasswordCurrent {
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String
}

#[derive(Deserialize, Debug)]
pub struct FieldChangePasswrod {
    pub user: Thing,
    pub token: String,
    pub new_password: Option<String>, // <- bikin opsional
    pub old_password: Option<String>,
    pub password_change_timestamp: Option<NaiveDateTime>,
    pub email: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataJsonResetPass {
    pub user_id: String,
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String,
    pub updated_at: String,
    pub hashed_new_password: String,
    pub hashed_old_password: String,
    pub email: String,
    pub token_verification: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataJsonCurrentPass {
    pub user_id: String,
    pub old_password: String,
    pub new_password: String,
    pub confirm_password: String,
    pub updated_at: String,
    pub hashed_new_password: String,
    pub hashed_old_password: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataPassword {
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataPasswordChange {
	pub id: Thing,
	pub user: Thing,
	pub durasi: Option<i32>,
	pub email: Option<String>,
	pub nama: Option<String>,
	pub new_password: Option<String>,
	pub old_password: Option<String>,
	pub password_change_timestamp: Option<String>,
	pub updated_at: String,
	pub created_at: String,
}
