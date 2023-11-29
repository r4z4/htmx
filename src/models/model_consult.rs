use chrono::{DateTime, Utc, serde::{ts_seconds_option, ts_seconds}};
use redis::{RedisResult, FromRedisValue, ErrorKind, Value, from_redis_value};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::SelectOption;

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultPost {
    #[validate(range(min = 1, max = 5, message = "Purpose out of range"))]
    pub consult_purpose_id: i32,
    pub client_id: i32,
    // Tired <Option>. Form makes it hard
    pub consultant_id: i32,
    #[validate(range(min = 1, message = "Location out of range"))]
    pub location_id: i32,
    pub attachment_path: Option<String>,
    pub linfa_assign: Option<String>,
    pub num_attendees: i32,
    pub consult_result_id: i32,
    pub consult_start_date: String,
    pub consult_start_time: String,
    pub consult_end_date: String,
    pub consult_end_time: String,
    // #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormRequest {
    pub client_id: i32,
    pub slug: String,
    pub consult_purpose_id: i32,
    pub consultant_id: Option<i32>,
    pub location_id: i32,
    pub consult_result_id: i32,
    pub consult_start: Option<DateTime<Utc>>,
    pub consult_end: Option<DateTime<Utc>>,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, Clone, Deserialize)]
pub struct ConsultList {
    // pub consult_id: i32,
    pub id: i32,
    pub slug: String,
    pub purpose: i32,
    // If using CONCAT or COALSCE likely need to make them Option<_>
    pub client_name: Option<String>,
    pub consultant_name: Option<String>,
    pub location_name: String,
    pub result: i32,
    #[serde(with = "ts_seconds")]
    pub consult_start: DateTime<Utc>,
    #[serde(with = "ts_seconds_option")]
    pub consult_end: Option<DateTime<Utc>>,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, Clone, Deserialize)]
pub struct ConsultListVec {
    pub vec: Vec<ConsultList>,
}

impl FromRedisValue for ConsultListVec {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let v: String = from_redis_value(v)?;
        let result: Self = match serde_json::from_str::<Self>(&v) {
          Ok(v) => v,
          Err(_err) => return Err((ErrorKind::TypeError, "Parse to JSON Failed").into())
        };
        Ok(result)
    }
}

#[derive(Debug, Validate, Serialize, FromRow, Clone, Deserialize)]
pub struct ConsultAttachments {
    // pub consult_id: i32,
    pub attachment_id: i32,
    pub short_desc: String,
    // If using CONCAT or COALSCE likely need to make them Option<_>
    pub mime_type_id: i32,
    pub path: String,
    // pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConsultListResponse {
    pub consults: Vec<ConsultList>,
    pub name: String,
}

#[derive(Serialize, Validate, Deserialize, Debug, Default, Clone)]
pub struct ConsultWithDates {
    pub consult_start_date: Option<String>,
    pub consult_start_time: Option<String>,
    pub consult_end_date: Option<String>,
    pub consult_end_time: Option<String>,
    pub consult_purpose_id: i32,
    pub slug: String,
    pub location_id: i32,
    pub consultant_id: Option<i32>,
    pub consult_result_id: i32,
    pub client_id: i32,
    #[validate(length(min = 3, message = "Notes must be greater than 3 chars"))]
    pub notes: Option<String>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultFormTemplate {
    pub entity: Option<ConsultWithDates>,
    pub location_options: Vec<SelectOption>,
    pub consultant_options: Vec<SelectOption>,
    pub client_options: Vec<SelectOption>,
    pub consult_result_options: Vec<SelectOption>,
    pub consult_purpose_options: Vec<SelectOption>,
}
