use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::user::UserModel;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Sessions {
    map: HashMap<String, UserModel>,
}
