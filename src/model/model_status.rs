use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing; // ✅ Gunakan Thing untuk ID

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusBody {
    pub status: String,
    pub reason: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataJsonStatusUsers {
    pub id: Thing, // penting!
    pub name: Option<String>, // mungkin perlu kalau nanti muncul
    pub email: Option<String>,
    pub user_description: String,
    pub contractor: Thing,
    pub created_at: Option<String>,
    pub project_id: Option<i32>,
    pub role_type: i32,
    pub is_verified: bool,
    pub status: Option<String>,
    pub reason: Option<String>
}