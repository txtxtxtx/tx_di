use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigQuery {
    pub name: Option<String>,
    pub category: Option<String>,
    pub config_key: Option<String>,
    pub config_type: Option<i32>,
}
