use test_context::{test_context, AsyncTestContext};

use crate::models::model_consultant::ConsultantPostRequest;

pub struct Context {
    pub sut: ConsultantPostRequest,
}

pub const GC_F_NAME: &str = "Greg";
pub const GC_L_NAME: &str = "Cote";
pub const GC_FULL_NAME: &str = "Greg Cote";

#[async_trait::async_trait]
impl AsyncTestContext for Context {
    async fn setup() -> Context {
        Context {
            sut: ConsultantPostRequest {
                user_id: 3,
                consultant_f_name: "Greg".to_string(),
                consultant_l_name: "Cote".to_string(),
                specialty_id: 1,
                territory_id: 1,
                img_path: None,
                // start_date: None,
                // end_date: None,
                // notes: None,
            },
        }
    }
    // fn teardown(self) {}
}
