use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use struct_iterable::Iterable;
use uuid::Uuid;
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
    // FIXME Make Uuid
    pub slug: String,
    pub location_name: String,
    pub location_address_one: String,
    pub location_address_two: Option<String>,
    pub location_city: String,
    pub location_zip: String,
    pub location_phone: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct LocationFormTemplate {
    pub entity: Option<LocationFormRequest>,
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
pub struct LocationFormRequest {
    pub location_name: String,
    pub slug: String,
    pub location_address_one: String,
    pub location_address_two: Option<String>,
    pub location_city: String,
    pub location_state: String,
    pub location_zip: String,
    pub location_contact_id: i32,
    pub location_phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Iterable)]
pub struct LocationPatchRequest {
    pub location_name: Option<String>,
    pub location_address_one: Option<String>,
    pub location_address_two: Option<Option<String>>,
    pub location_city: Option<String>,
    pub location_state: Option<String>,
    pub location_zip: Option<String>,
    pub location_contact_id: Option<i32>,
    pub location_phone: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LocationPostResponse {
    pub location_id: i32,
}
