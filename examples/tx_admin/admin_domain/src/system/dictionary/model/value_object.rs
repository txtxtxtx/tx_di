use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DictTypeQuery {
    pub name: Option<String>,
    pub dict_type: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DictDataQuery {
    pub dict_type: Option<String>,
    pub label: Option<String>,
    pub status: Option<i32>,
}
