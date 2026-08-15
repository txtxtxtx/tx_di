use serde::{Deserialize, Serialize};

/// Role query filters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoleQuery {
    pub name: Option<String>,
    pub code: Option<String>,
    pub status: Option<i32>,
}

/// Role simple info for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleSimple {
    pub id: u64,
    pub name: String,
    pub code: String,
}
