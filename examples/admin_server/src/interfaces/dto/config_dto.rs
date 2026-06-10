//! 配置 DTO

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ConfigDto {
    pub id: u64,
    pub category: Option<String>,
    pub config_type: String,
    pub name: String,
    pub config_key: String,
    pub value: Option<String>,
    pub visible: bool,
    pub remark: Option<String>,
    pub creator: Option<String>,
    pub created_at: String,
    pub updater: Option<String>,
    pub updated_at: String,
}

impl From<&crate::domain::config::Config> for ConfigDto {
    fn from(c: &crate::domain::config::Config) -> Self {
        Self {
            id: c.id,
            category: c.category.clone(),
            config_type: c.config_type.to_string(),
            name: c.name.clone(),
            config_key: c.config_key.clone(),
            value: c.value.clone(),
            visible: c.visible,
            remark: c.remark.clone(),
            creator: c.creator.clone(),
            created_at: c.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
            updater: c.updater.clone(),
            updated_at: c.updated_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateConfigRequest {
    pub category: Option<String>,
    pub name: String,
    pub config_key: String,
    pub value: Option<String>,
    pub visible: Option<bool>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub category: Option<String>,
    pub name: Option<String>,
    pub config_key: Option<String>,
    pub value: Option<String>,
    pub visible: Option<bool>,
    pub remark: Option<String>,
}
