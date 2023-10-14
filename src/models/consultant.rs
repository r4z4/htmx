use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::{ResponseConsultant, SelectOptions};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsultantPostRequest {
    pub consultant_f_name: String,
    pub consultant_l_name: String,
    pub specialty_id: i32,
    pub territory_id: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseConsultantList {
    pub consultants: Vec<ResponseConsultant>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConsultantListResponse {
    pub consultants: Vec<ResponseConsultant>,
    pub name: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultantList {
    pub consult_id: i32,
    pub client_id: i32,
    pub consultant_id: i32,
    pub location_id: i32,
    pub created_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultantFormTemplate {
    pub account_options: Vec<SelectOptions>,
}
