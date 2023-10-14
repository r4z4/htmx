use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::SelectOptions;

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationPostRequest {
    pub location_name: String,
    pub specialty_id: i32,
    pub territory_id: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseLocationList {
    pub locations: Vec<LocationList>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LocationListResponse {
    pub locations: Vec<LocationList>,
    pub name: String,
}

#[derive(Debug, Validate, Serialize, Clone, FromRow, Deserialize)]
pub struct LocationList {
    pub location_id: i32,
    pub location_name: String,
    pub location_address_one: String,
    pub location_address_two: Option<String>,
    pub location_city: String,
    pub location_zip: String,
    pub location_phone: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct LocationFormTemplate {
    pub account_options: Vec<SelectOptions>,
}
