use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::SelectOption;

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultPost {
    pub consult_purpose_id: i32,
    pub client_id: i32,
    pub consultant_id: i32,
    pub location_id: i32,
    pub attachment_path: Option<String>,
    pub consult_result_id: i32,
    pub consult_start_date: String,
    pub consult_start_time: String,
    pub consult_end_date: String,
    pub consult_end_time: String,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormRequest {
    pub client_id: i32,
    pub slug: String,
    pub consult_purpose_id: i32,
    pub consultant_id: Option<i32>,
    pub location_id: i32,
    pub consult_result_id: i32,
    pub consult_start: Option<DateTime<Utc>>,
    pub consult_end: Option<DateTime<Utc>>,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, Clone, Deserialize)]
pub struct ConsultList {
    // pub consult_id: i32,
    pub id: i32,
    pub slug: String,
    pub consult_purpose_id: i32,
    // If using CONCAT or COALSCE likely need to make them Option<_>
    pub client_name: Option<String>,
    pub consultant_name: Option<String>,
    pub location_name: String,
    pub consult_result_id: i32,
    pub consult_start: DateTime<Utc>,
    pub consult_end: Option<DateTime<Utc>>,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct ConsultAttachments {
    // pub consult_id: i32,
    pub attachment_id: i32,
    pub short_desc: String,
    // If using CONCAT or COALSCE likely need to make them Option<_>
    pub mime_type_id: i32,
    pub path: String,
    // pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConsultListResponse {
    pub consults: Vec<ConsultList>,
    pub name: String,
}

#[derive(Serialize, Validate, Deserialize, Debug, Default, Clone)]
pub struct ConsultWithDates {
    pub consult_start_date: Option<String>,
    pub consult_start_time: Option<String>,
    pub consult_end_date: Option<String>,
    pub consult_end_time: Option<String>,
    pub consult_purpose_id: i32,
    pub slug: String,
    pub location_id: i32,
    pub consultant_id: Option<i32>,
    pub consult_result_id: i32,
    pub client_id: i32,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormTemplate {
    pub entity: Option<ConsultWithDates>,
    pub location_options: Vec<SelectOption>,
    pub consultant_options: Vec<SelectOption>,
    pub client_options: Vec<SelectOption>,
    pub consult_result_options: Vec<SelectOption>,
    pub consult_purpose_options: Vec<SelectOption>,
}
