use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CountPagination {
	pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub total_pages: i64,
    pub count_per_page: i32,
}

#[derive(Deserialize, Debug)]
pub struct ParamPagination {
    pub page: i32,
    pub limit: i32,
}