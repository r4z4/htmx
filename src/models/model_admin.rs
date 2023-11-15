use crate::config::{validate_primary_address, validate_secondary_address, validate_username};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Encode, FromRow};
use validator::Validate;

use crate::config::{SelectOption, StringSelectOption};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserList {
    pub user_id: i32,
    pub slug: String,
    pub username: String,
    pub user_type_id: i32,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub avatar_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminSubadminFormTemplate {
    pub user_type_id: i32,
    pub username: String,
    pub email: String,
    pub address_one: String,
    pub address_two: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub primary_phone: String,
    pub updated_at_fmt: String,
    pub avatar_path: Option<String>,
    pub user_type_options: Vec<SelectOption>,
    pub state_options: Vec<StringSelectOption>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserFormTemplate {
    pub user_type_id: i32,
    pub username: String,
    pub email: String,
    pub updated_at_fmt: String,
    pub avatar_path: Option<String>,
    pub user_type_options: Vec<SelectOption>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminSubadminFormQuery {
    pub username: String,
    pub email: String,
    pub address_one: String,
    pub address_two: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub primary_phone: String,
    pub user_type_id: i32,
    pub updated_at: Option<DateTime<Utc>>,
    pub avatar_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserFormQuery {
    pub username: String,
    pub email: String,
    pub user_type_id: i32,
    pub updated_at: Option<DateTime<Utc>>,
    pub avatar_path: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, Default, Clone)]
pub struct AdminSubadminPostRequest {
    pub user_type_id: i32,
    #[validate(custom = "validate_username")]
    pub username: String,
    pub email: String,
    #[validate(custom(
        function = "validate_primary_address",
        message = "Primary Address is improperly formatted"
    ))]
    pub address_one: String,
    #[validate(custom = "validate_secondary_address")]
    pub address_two: Option<String>,
    #[validate(length(max = 28, message = "City must be less than 28 chars"))]
    pub city: String,
    pub state: String,
    #[validate(length(equal = 5, message = "Zip must be 5 chars"))]
    pub zip: String,
    pub primary_phone: String,
}

#[derive(Serialize, Deserialize, Validate, Debug, Default, Clone)]
pub struct AdminUserPostRequest {
    pub user_type_id: i32,
    #[validate(custom = "validate_username")]
    pub username: String,
    pub email: String,
    pub avatar_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow, Encode)]
pub struct AdminUserPostResponse {
    pub user_id: i32,
}
