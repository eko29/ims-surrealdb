use actix_web::{web, HttpResponse, Responder};
use serde_json::{json, Value};

use crate::{
    model_user::{DataJsonUsers, ParamAllUser, CountResult},
    sukses_response::ApiResponse, 
    AppState
};

pub async fn get_all_user(
    app_state: web::Data<AppState>,
    query: web::Query<ParamAllUser>,
) -> impl Responder {
    let db = &app_state.db;
    let query = query.into_inner();
    let is_delete = false;

    println!("query get {:?}", query);
    let offset = (query.page - 1) * query.limit;

    // Start constructing query
    let mut query_str = String::from("SELECT * FROM m_user WHERE true");
    let mut binds: Vec<(&str, Value)> = vec![];

    if let Some(role_type) = query.role_type {
        query_str.push_str(" AND role_type = $role_type");
        binds.push(("role_type", role_type.into())); // i32 -> Value
    }

    if let Some(contractor_id) = query.contractor_id {
        let contractor_id_str = format!("contractor:{}", contractor_id);
        query_str.push_str(" AND contractor = $contractor_id");
        binds.push(("contractor_id", contractor_id_str.into())); // String -> Value
    }

    if let Some(project_id) = query.project_id {
        query_str.push_str(" AND project_id = $project_id");
        binds.push(("project_id", project_id.into())); // i32 -> Value
    }

    if let Some(search) = &query.search {
        let search_lower = search.to_lowercase();  // Pencarian tidak peka huruf besar/kecil
        query_str.push_str(" AND (name = $search OR email = $search)");
        binds.push(("search", format!("{}", search_lower).into())); // Menambahkan wildcard (%)
    }

    // Gabungkan kondisi is_delete dengan klausa WHERE yang sudah ada
    query_str.push_str(" AND is_delete = $is_delete");

    // Setelah klausa WHERE, tambahkan ORDER BY
    query_str.push_str(&format!(" ORDER BY {} {}", query.sort, query.order.to_uppercase()));

    // Kemudian tambahkan LIMIT dan OFFSET
    query_str.push_str(" LIMIT $limit START $offset");

    binds.push(("is_delete", serde_json::Value::Bool(is_delete.clone())));
    binds.push(("limit", query.limit.into()));
    binds.push(("offset", offset.into()));

    println!("Final Query: {}", query_str);

    let mut surreal_query = db.query(query_str);
    for (key, value) in &binds {
        surreal_query = surreal_query.bind((*key, value.clone())); // bind by reference, clone values
    }

    // ===== Build query for count ===== RETURN { count: count((SELECT id FROM m_user WHERE true)) }
    let mut count_query = String::from("RETURN { count: count((SELECT id FROM m_user WHERE true");

    // Salin binds untuk digunakan di query count
    let count_binds = binds.clone();

    // Tambahkan kondisi is_delete ke dalam query count
    count_query.push_str(" AND is_delete = $is_delete");

    for (key, _) in &count_binds {
        match *key {
            "role_type" => count_query.push_str(" AND role_type = $role_type"),
            "contractor_id" => count_query.push_str(" AND contractor = $contractor_id"),
            "project_id" => count_query.push_str(" AND project_id = $project_id"),
            "search" => count_query.push_str(" AND (name IS NOT NULL AND string::lowercase(name) CONTAINS string::lowercase($search) OR email IS NOT NULL AND string::lowercase(email) CONTAINS string::lowercase($search))"),
            _ => {}
        }
    }

    // Tutup SELECT
    count_query.push_str("))}");

    println!("🧠 Final Count Query: {}", count_query);
    println!("🔗 Binds for Count Query: {:?}", count_binds);

    let mut surreal_count_query = db.query(count_query);
    for (key, value) in &count_binds {
        surreal_count_query = surreal_count_query.bind((*key, value.clone())); // bind by reference, clone values
    }

    
    // ===== Execute both queries =====
    let users_result = surreal_query.await;
    let count_result = surreal_count_query.await;

    println!("count_result {:?}", count_result);

    match (users_result, count_result) {
        (Ok(mut users_response), Ok(mut count_response)) => {
            let users: Vec<DataJsonUsers> = users_response.take(0).unwrap_or_default();
            
            let count: i64 = match count_response.take::<Vec<CountResult>>(0) {
                Ok(counts) => {
                    println!("✅ Count records: {:?}", counts);
                    counts.get(0).map(|c| c.count).unwrap_or(0)
                },
                Err(e) => {
                    println!("❌ Failed to take count from response: {}", e);
                    0
                }
            };
            // let count = 7;
            println!("total record {}", count);
            let total_pages = if count > 0 {
                (count as f64 / query.limit as f64).ceil() as i64
            } else {
                0
            };
            let count_per_page = users.len();

            let pagination = json!({
                "total": count,
                "page": query.page,
                "limit": query.limit,
                "total_pages": total_pages,
                "count_per_page": count_per_page
            });

            let data = json!({
                "user": users,
                "pagination": pagination
            });

            let api_response = ApiResponse::new(200, "success".to_string(), data);
            HttpResponse::Ok().json(api_response)
        }

        (Err(e), _) | (_, Err(e)) => {
            HttpResponse::InternalServerError().body(format!("Database error: {}", e))
        }
    }
}

