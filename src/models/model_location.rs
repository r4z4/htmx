use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use struct_iterable::Iterable;
use validator::{Validate, ValidationError};

use crate::config::{SelectOption, StringSelectOption};
use crate::config::{validate_primary_address, validate_secondary_address};
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
    #[validate(custom = "validate_primary_address")]
    #[validate(contains = " ")]
    pub location_address_one: String,
    #[validate(custom = "validate_secondary_address")]
    pub location_address_two: Option<String>,
    #[validate(length(
        min = 2,
        max = 28,
        message = "City must be between 2 & 28 chars"
    ))]
    pub location_city: String,
    // #[validate(length(min = 3, message = "Must be in list of states"))]
    pub location_state: String,
    #[validate(length(equal = 5, message = "Zip must be 5 chars"))]
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

#[derive(Debug, Serialize, Validate, Deserialize, Iterable)]
pub struct LocationPatchRequest {
    #[validate(length(min = 3, message = "Location Name must be greater than 2 chars"))]
    pub location_name: Option<String>,
    #[validate(custom = "validate_primary_address")]
    pub location_address_one: Option<String>,
    #[validate(custom = "validate_secondary_address")]
    pub location_address_two: Option<Option<String>>,
    #[validate(length(
        min = 2,
        max = 28,
        message = "City must be between 2 & 28 chars"
    ))]
    pub location_city: Option<String>,
    pub location_state: Option<String>,
    #[validate(length(equal = 5, message = "Zip must be 5 chars"))]
    pub location_zip: Option<String>,
    pub location_contact_id: Option<i32>,
    #[validate(length(equal = 12, message = "Phone must be 12 characters (w/ -)"))]
    pub location_phone: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LocationPostResponse {
    pub location_id: i32,
}
