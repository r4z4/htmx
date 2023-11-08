use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use struct_iterable::Iterable;
use validator::Validate;
use crate::config::{validate_primary_address, validate_secondary_address};
use crate::config::{SelectOption, StringSelectOption};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseClientList {
    pub clients: Vec<ClientList>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ClientListResponse {
    pub clients: Vec<ClientList>,
    pub name: String,
}

#[derive(Debug, Validate, Serialize, Clone, FromRow, Deserialize)]
pub struct ClientList {
    pub client_id: i32,
    pub slug: String,
    pub specialty_name: String,
    pub client_company_name: Option<String>,
    pub client_f_name: Option<String>,
    pub client_l_name: Option<String>,
    pub client_email: String,
    pub client_address_one: String,
    pub client_address_two: Option<String>,
    pub client_city: String,
    pub client_zip: String,
    pub client_primary_phone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Validate, Iterable)]
pub struct ClientPostRequest {
    pub client_f_name: Option<String>,
    pub client_l_name: Option<String>,
    pub client_company_name: Option<String>,
    #[validate(custom = "validate_primary_address")]
    pub client_address_one: String,
    #[validate(custom = "validate_secondary_address")]
    pub client_address_two: Option<String>,
    pub account_id: i32,
    pub specialty_id: i32,
    #[validate(email)]
    pub client_email: String,
    #[validate(length(
        min = 2,
        max = 28,
        message = "City must be between 2 & 28 chars"
    ))]
    pub client_city: String,
    pub client_state: String,
    #[validate(length(equal = 3, message = "Zip must be 5 chars"))]
    pub client_zip: String,
    pub client_dob: Option<String>,
    #[validate(length(equal = 12, message = "Phone must be 12 characters (w/ -)"))]
    pub client_primary_phone: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ClientFormRequest {
    pub client_company_name: Option<String>,
    pub client_f_name: Option<String>,
    pub client_l_name: Option<String>,
    pub slug: String,
    pub account_id: i32,
    pub specialty_id: i32,
    pub client_address_one: String,
    pub client_address_two: Option<String>,
    pub client_dob: Option<NaiveDate>,
    pub client_city: String,
    pub client_state: String,
    pub client_zip: String,
    pub client_email: String,
    pub client_primary_phone: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ClientFormTemplate {
    pub entity: Option<ClientFormRequest>,
    pub specialty_options: Vec<SelectOption>,
    pub state_options: Vec<StringSelectOption>,
    pub account_options: Vec<SelectOption>,
}

#[derive(Serialize, Validate, Deserialize, Debug, Default, Clone)]
pub struct ClientWithDates {
    pub client_company_name: Option<String>,
    pub client_f_name: Option<String>,
    pub client_l_name: Option<String>,
    pub client_address_one: String,
    pub client_address_two: Option<String>,
    pub client_city: String,
    pub client_state: String,
    pub client_zip: String,
    pub client_dob: Option<String>,
    pub slug: String,
    pub client_primary_phone: String,
    pub client_email: String,
    pub account_id: i32,
    pub specialty_id: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ClientPostResponse {
    pub client_id: i32,
}
