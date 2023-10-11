use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    user_id: i32,
    username: String,
    // first_name: Option<String>,
    // last_name: Option<String>,
    email: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // user_type: UserType,
}

/// An admin is still a user
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Admin(UserModel);