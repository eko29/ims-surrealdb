use serde::{Deserialize, Serialize, Deserializer};
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

#[derive(Deserialize)]
pub struct ExportQuery {
    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub role_type: Option<i32>,

    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub contractor_id: Option<i32>,

    #[serde(deserialize_with = "empty_string_as_none")]
    #[serde(default)] 
    pub project_id: Option<i32>,

    pub format: String,
}
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BodyFormJson {
    pub name: String,
    pub email: String,
    pub user_description: String,
    pub contractor: i32,
    pub project_id: i32,
    pub role_type: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportFormJson {
    pub name: Option<String>,
    pub email: Option<String>,
    pub user_description: Option<String>,
    pub contractor: Option<Thing>,
    pub project_id: Option<i32>,
    pub role_type: Option<i32>,
    pub is_delete: bool,
    pub is_verified: bool,
    pub status: Option<String>
}

#[derive(Serialize)]
pub struct ExportUserRow {
    pub name: String,
    pub email: String,
    pub user_description: String,
    pub contractor: String,
    pub project_id: String,
    pub role_type: String,
    pub is_delete: bool,
    pub is_verified: bool,
    pub status: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FormJson {
    pub file: String,
    pub send_verification_email: bool,
    pub update_existing: bool,
}

#[derive(Serialize)]
pub struct ImportFailedRecord {
    pub row: usize,
    pub email: String,
    pub error: String,
}

#[derive(Serialize)]
pub struct ImportResponse {
    pub total_records: usize,
    pub created: usize,
    pub updated: usize,
    pub failed: usize,
    pub failed_records: Vec<ImportFailedRecord>,
}