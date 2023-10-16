use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::{SelectOption, StringSelectOption};


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
    pub state_options: Vec<StringSelectOption>,
    pub location_contact_options: Vec<SelectOption>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LocationPostRequest {
    pub location_name: String,
    pub location_address_one: String,
    pub location_address_two: Option<String>,
    pub location_city: String,
    pub location_state: String,
    pub location_zip: String,
    pub location_contact_id: i32,
    pub location_phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LocationPostResponse {
    pub location_id: i32,
}
