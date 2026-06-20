use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobQuery {
    pub name: Option<String>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobLogQuery {
    pub job_id: Option<u64>,
    pub status: Option<i32>,
}
