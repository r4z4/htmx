use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{SelectOption, StringSelectOption};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserList {
    pub user_id: i32, 
    pub username: String, 
    pub email: String, 
    pub created_at: DateTime<Utc>, 
    pub avatar_path: String
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserFormTemplate {
    pub user_type_id: i32, 
    pub username: String, 
    pub email: String, 
    pub updated_at_fmt: String,
    pub avatar_path: String,
    pub user_type_options: Vec<SelectOption>,
    pub state_options: Vec<StringSelectOption>
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AdminUserFormQuery {
    pub username: String, 
    pub email: String, 
    pub user_type_id: i32,
    pub updated_at: Option<DateTime<Utc>>, 
    pub avatar_path: String
}

pub struct AdminUserPostRequest {

}