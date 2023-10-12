use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::config::SelectOptions;

// #[derive(Serialize, Default, Deserialize, Debug, Clone, PartialEq)]
// // #[serde(rename_all = "camelCase")]
// enum UserType {
//     #[default]
//     Guest,
//     RegularUser,
//     Admin
// }

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
// #[serde(rename_all = "camelCase")]
pub struct UserModel {
    pub user_id: i32,
    pub username: String,
    // pub first_name: Option<String>,
    // pub last_name: Option<String>,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // user_type: UserType,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct UserSettingsModel {
    pub theme_options: Vec<SelectOptions>,
    pub email: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, FromRow)]
pub struct UserHomeModel {
    pub created_at: DateTime<Utc>,
    pub email: String,
    pub username: String,
}

/// An admin is still a user
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Admin(UserModel);