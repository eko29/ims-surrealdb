use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{self};
use std::str::FromStr;
use surrealdb::sql::Thing; // ✅ Gunakan Thing untuk ID

fn empty_string_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(ref s) if s.trim().is_empty() => Ok(None), // Jika string kosong, return None
        Some(s) => s.parse().map(Some).map_err(de::Error::custom), // Parse string ke tipe T
        None => Ok(None), // Jika None, return None
    }
}

#[derive(Debug, Deserialize)]
pub struct ParamAllUser {
    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub role_type: Option<i32>,

    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub contractor_id: Option<i32>,

    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub project_id: Option<i32>,

    pub page: i32,
    pub limit: i32,

    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub search: Option<String>,

    pub sort: String,
    pub order: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJsonUsers {
    pub id: Thing, // penting!
    pub name: Option<String>, // mungkin perlu kalau nanti muncul
    pub email: Option<String>,
    pub user_description: String,
    pub contractor: Thing,
    pub created_at: Option<String>,
    pub project_id: Option<i32>,
    pub role_type: i32,
    pub is_verified: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJsonUsersAll {
    pub id: Thing, // penting!
    pub name: Option<String>, // mungkin perlu kalau nanti muncul
    pub email: String,
    pub user_description: String,
    pub contractor: Thing,
    pub created_at: Option<String>,
    pub project_id: Option<i32>,
    pub role_type: i32,
    pub is_verified: bool,
    pub password: String
}

#[derive(Debug, Deserialize)]
pub struct CountResult {
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BodyJsonUser {
    pub name: Option<String>, // mungkin perlu kalau nanti muncul
    pub email: Option<String>,
    pub user_description: String,
    pub contractor: Thing,
    pub updated_at: Option<String>,
    pub project_id: Option<i32>,
    pub role_type: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BodyJsonProTy {
    pub project_id: i32,
    pub role_type: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJsonName {
    pub name: String, 
    pub user_description: String
}