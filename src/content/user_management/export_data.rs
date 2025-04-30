use actix_web::{web, HttpResponse, Error};
use surrealdb::sql::Value;

use crate::{
    error_response::{ApiErrorType, ApiResponseError}, 
    model_export_import::{ExportFormJson, ExportQuery, ExportUserRow}, 
    AppState
};
use std::{fmt::Write, io::Cursor};

use csv::WriterBuilder;
use umya_spreadsheet::{
    new_file, 
    writer::xlsx::write_writer
};


pub async fn export_users(
    app_state: web::Data<AppState>,
    query: web::Query<ExportQuery>,
) -> Result<HttpResponse, Error> {
    let db = &app_state.db;

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

    println!("Final Query: {}", query_str);

    let mut surreal_query = db.query(query_str);
    for (key, value) in &binds {
        surreal_query = surreal_query.bind((*key, value.clone())); // bind by reference, clone values
    }

    let results = surreal_query.await;


    let users: Vec<ExportFormJson> = match results {
        Ok(mut res) => match res.take::<Vec<ExportFormJson>>(0) {
            Ok(data) => {
                // Menambahkan println untuk memeriksa data yang diambil
                println!("hasil print out: {:?}", &data);
                data // mengembalikan data setelah print
            },
            Err(e) => {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    format!("Gagal mengambil hasil query: {}", e),
                );
                return Ok(HttpResponse::InternalServerError().json(err));
            }
        },
        Err(e) => {
            let err = ApiResponseError::new(
                ApiErrorType::InternalServerError,
                format!("Gagal query user: {}", e),
            );
            return Ok(HttpResponse::InternalServerError().json(err));
        }
    };

    // Validasi panjang data, misalnya memastikan 6 kolom seperti yang diharapkan
    if users.is_empty() {
        let err = ApiResponseError::new(
            ApiErrorType::InternalServerError,
            "Tidak ada data yang ditemukan".to_string(),
        );
        return Ok(HttpResponse::InternalServerError().json(err));
    }

    match query.format.as_str() {
        "csv" => {
            let rows: Vec<ExportUserRow> = users.iter().map(|user| {
                ExportUserRow {
                    name: user.name.clone().unwrap_or_default(),
                    email: user.email.clone().unwrap_or_default(),
                    user_description: user.user_description.clone().unwrap_or_default(),
                    contractor: user.contractor.as_ref().map_or(String::new(), |c| c.id.to_string()),
                    project_id: user.project_id.map_or(String::new(), |id| id.to_string()),
                    role_type: user.role_type.map_or(String::new(), |id| id.to_string()),
                    is_delete: user.is_delete,
                    is_verified: user.is_verified,
                    status: Some(user.status.clone().unwrap_or_default())

                }
            }).collect();

            let mut wtr = WriterBuilder::new().from_writer(vec![]);
            for row in &rows {
                wtr.serialize(row).unwrap();
            }
            
            let csv_data = match wtr.into_inner() {
                Ok(data) => data,
                Err(e) => {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal menyelesaikan penulisan CSV: {}", e),
                    );
                    return Ok(HttpResponse::InternalServerError().json(err));
                }
            };
            
            Ok(HttpResponse::Ok()
                .insert_header(("Content-Type", "text/csv"))
                .insert_header(("Content-Disposition", "attachment; filename=\"users.csv\""))
                .body(csv_data))
        }

        "txt" => {
            let mut txt_data = String::new();
            for user in &users {
                let line = format!(
                    "{},{},{},{},{},{},{},{},{}\n",
                    user.name.clone().unwrap_or_else(|| "".to_string()),
                    user.email.clone().unwrap_or_else(|| "".to_string()),
                    user.user_description.clone().unwrap_or_else(|| "".to_string()),
                    user.contractor.as_ref().map_or("".to_string(), |c| c.id.to_string()),
                    user.project_id.map_or("".to_string(), |p| p.to_string()),
                    user.role_type.map_or("".to_string(), |p| p.to_string()),
                    user.is_delete,
                    user.is_verified,
                    user.status.clone().unwrap_or_default()
                );
                // txt_data.push_str(&line);
                if let Err(e) = txt_data.write_str(&line) {
                    let err = ApiResponseError::new(
                        ApiErrorType::InternalServerError,
                        format!("Gagal menulis data ke file txt: {}", e),
                    );
                    return Ok(HttpResponse::InternalServerError().json(err));
                }
            }
            Ok(HttpResponse::Ok()
                .insert_header(("Content-Type", "text/plain"))
                .insert_header(("Content-Disposition", "attachment; filename=\"users.txt\""))
                .body(txt_data))
        }

        "xlsx" => {
            let mut book = new_file();
            let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();

            // Header
            sheet.get_cell_mut("A1").set_value("Name");
            sheet.get_cell_mut("B1").set_value("Email");
            sheet.get_cell_mut("C1").set_value("User Description");
            sheet.get_cell_mut("D1").set_value("Contractor");
            sheet.get_cell_mut("E1").set_value("Project ID");
            sheet.get_cell_mut("F1").set_value("Role Type");
            sheet.get_cell_mut("G1").set_value("Is Delete");
            sheet.get_cell_mut("H1").set_value("Is Verified");
            sheet.get_cell_mut("I1").set_value("Status");

            for (i, user) in users.iter().enumerate() {
                let row = i + 2;
                sheet.get_cell_mut(format!("A{}", row)).set_value(&user.name.clone().unwrap_or_else(|| "".to_string()));
                sheet.get_cell_mut(format!("B{}", row)).set_value(&user.email.clone().unwrap_or_else(|| "".to_string()));
                sheet.get_cell_mut(format!("C{}", row)).set_value(&user.user_description.clone().unwrap_or_else(|| "".to_string()));
                sheet.get_cell_mut(format!("D{}", row)).set_value(&user.contractor.as_ref().map_or(String::new(), |c| c.id.to_string()));
                sheet.get_cell_mut(format!("E{}", row)).set_value(&user.project_id.map_or(String::new(), |id| id.to_string()));
                sheet.get_cell_mut(format!("F{}", row)).set_value(user.role_type.map_or(String::new(), |id| id.to_string()));
                sheet.get_cell_mut(format!("G{}", row)).set_value(user.is_delete.to_string());
                sheet.get_cell_mut(format!("H{}", row)).set_value(user.is_verified.to_string());
                sheet.get_cell_mut(format!("I{}", row)).set_value(&user.status.clone().unwrap_or_default());
            }

            // Tulis ke buffer (memori)
            let mut buffer = Cursor::new(Vec::new());
            // write_writer(&book, &mut buffer).unwrap();
            if let Err(e) = write_writer(&book, &mut buffer) {
                let err = ApiResponseError::new(
                    ApiErrorType::InternalServerError,
                    format!("Gagal menulis file XLSX: {}", e),
                );
                return Ok(HttpResponse::InternalServerError().json(err));
            }

            Ok(HttpResponse::Ok()
                .insert_header(("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"))
                .insert_header(("Content-Disposition", "attachment; filename=\"users.xlsx\""))
                .body(buffer.into_inner()))
        }

        _ => {
            let err = ApiResponseError::new(
                ApiErrorType::BadRequest,
                format!("Format tidak didukung: gunakan csv, txt, atau xlsx"),
            );
            return Ok(HttpResponse::BadRequest().json(err));
            // Ok(HttpResponse::BadRequest().json()),
        }
    }
}


