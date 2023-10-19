use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::SelectOption;

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultPost {
    pub client_id: i32,
    pub consultant_id: i32,
    pub location_id: i32,
    pub consult_start: DateTime<Utc>,
    pub consult_end: Option<DateTime<Utc>>,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormRequest {
    pub client_id: i32,
    pub consultant_id: i32,
    pub location_id: i32,
    pub consult_start: DateTime<Utc>,
    pub consult_end: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct ConsultList {
    pub consult_id: i32,
    pub slug: String,
    pub client_id: i32,
    pub consultant_id: i32,
    pub location_id: i32,
    pub consult_start: DateTime<Utc>,
    pub consult_end: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConsultListResponse {
    pub consults: Vec<ConsultList>,
    pub name: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormTemplate {
    pub entity: Option<ConsultFormRequest>,
    pub location_options: Vec<SelectOption>,
    pub consultant_options: Vec<SelectOption>,
    pub client_options: Vec<SelectOption>,
}
