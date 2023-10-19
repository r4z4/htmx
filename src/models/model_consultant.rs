use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

use crate::config::SelectOption;
#[derive(Debug, Serialize, Deserialize)]
pub struct ConsultantPostRequest {
    pub consultant_f_name: String,
    pub consultant_l_name: String,
    pub specialty_id: i32,
    pub territory_id: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseConsultantList {
    pub consultants: Vec<ResponseConsultant>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ConsultantListResponse {
    pub consultants: Vec<ResponseConsultant>,
    pub name: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultantList {
    pub consultant_id: i32,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ResponseConsultant {
    pub consultant_id: i32,
    pub slug: String,
    pub specialty_name: String,
    pub territory_name: String,
    pub consultant_f_name: String,
    pub consultant_l_name: String,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultantFormTemplate {
    pub account_options: Vec<SelectOption>,
    pub territory_options: Vec<SelectOption>,
    pub specialty_options: Vec<SelectOption>,
    pub admin_user_options: Vec<SelectOption>,
}

#[derive(Debug, Validate, Serialize, FromRow, Deserialize)]
pub struct ConsultantFormRequest {
    pub consultant_f_name: String,
    pub consultant_l_name: String,
    pub slug: String,
    pub specialty_id: i32,
    pub territory_id: i32,
    pub img_path: Option<String>,
}

impl ConsultantPostRequest {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.consultant_f_name, self.consultant_l_name)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_common::{*, self};
    use test_context::{test_context, TestContext};

    #[test_context(Context)]
    #[test]
    fn full_name_is_first_name_space_last_name(ctx: &mut Context) {
        let full_name = ctx.sut.full_name();
        // Assert
        assert_eq!(full_name, test_common::GC_FULL_NAME, "Unexpected full_name");
    }
}
