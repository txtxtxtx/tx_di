use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConfigCommand {
    pub category: String,
    pub config_type: i32,
    pub name: String,
    pub config_key: String,
    pub value: String,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigCommand {
    pub config_id: u64,
    pub category: String,
    pub config_type: i32,
    pub name: String,
    pub config_key: String,
    pub value: String,
    pub visible: i32,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigQueryRequest {
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub category: Option<String>,
    #[serde(deserialize_with = "crate::empty_string::deserialize_optional_string", default)]
    pub config_key: Option<String>,
    pub config_type: Option<i32>,
    pub page: i64,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConfigResponse {
    pub id: u64,
    pub category: String,
    pub config_type: i32,
    pub name: String,
    pub config_key: String,
    pub value: String,
    pub visible: i32,
    pub remark: Option<String>,
}

impl From<admin_domain::config::model::aggregate::Config> for ConfigResponse {
    fn from(config: admin_domain::config::model::aggregate::Config) -> Self {
        Self {
            id: config.id,
            category: config.category,
            config_type: config.config_type,
            name: config.name,
            config_key: config.config_key,
            value: config.value,
            visible: config.visible,
            remark: config.remark,
        }
    }
}
