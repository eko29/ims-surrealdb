use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    code: u32,
    response: String,
    data: T, // Make the data field public
}

impl<T> ApiResponse<T> {
    pub fn new(code: u32, response: String, data: T) -> Self {
        Self {
            code,
            response,
            data,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponseSukses {
    pub code: u32,
    pub response: String,
    pub pesan: String,
}

impl ApiResponseSukses {
    pub fn new(code: u32, response: String, pesan: String) -> Self {
        Self {
            code,
            response,
            pesan,
        }
    }
}