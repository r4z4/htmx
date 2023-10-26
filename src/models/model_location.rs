use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use struct_iterable::Iterable;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::config::{SelectOption, StringSelectOption, ACCEPTED_PRIMARIES};

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

fn validate_unique_location_name(location_name: &str) -> Result<(), ValidationError> {
    println!("Validating {}", location_name);
    if location_name == "Terrible Location" {
        // the value of the username will automatically be added later
        return Err(ValidationError::new("terrible_location"));
    }

    Ok(())
}

fn validate_primary_addr(location_address_one: &str) -> Result<(), ValidationError> {
    let street_strings: Vec<&str> = location_address_one
        .split(" ")
        .collect::<Vec<&str>>()
        .to_owned();
    let ss_len = street_strings.len();
    // Getting last two to account for 101 Hartford St. W etc..
    if ACCEPTED_PRIMARIES.contains(&street_strings[ss_len - 1])
        || ACCEPTED_PRIMARIES.contains(&street_strings[ss_len - 2])
    {
        Ok(())
    } else {
        Err(ValidationError::new(
            "Primary Address does not contain a valid Identifier (St, Ave)",
        ))
    }
}

#[derive(Debug, Validate, Serialize, Deserialize, FromRow)]
pub struct LocationPostRequest {
    // #[validate(length(min = 3, message = "Location Name must be greater than 2 chars"), custom = "validate_unique_location_name")]
    #[validate(length(min = 3, message = "Location Name must be greater than 2 chars"))]
    #[validate(custom(
        function = "validate_unique_location_name",
        code = "loc_name",
        message = "Don't use that name, it's terrible!"
    ))]
    pub location_name: String,
    #[validate(
        length(min = 3, message = "Location Address must ..."),
        custom = "validate_primary_addr"
    )]
    #[validate(contains = " ")]
    pub location_address_one: String,
    pub location_address_two: Option<String>,
    #[validate(length(
        min = 2,
        max = 28,
        message = "Location Address must be between 2 & 28 chars"
    ))]
    pub location_city: String,
    // #[validate(length(min = 3, message = "Must be in list of states"))]
    pub location_state: String,
    // #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub location_zip: String,
    pub location_contact_id: i32,
    #[validate(length(equal = 12, message = "Phone must be 12 characters (w/ -)"))]
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
