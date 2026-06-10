use serde::{Deserialize, Serialize};

/// Permission type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionType {
    /// Menu permission
    Menu = 0,
    /// Button permission
    Button = 1,
    /// API permission
    Api = 2,
}

impl From<i32> for PermissionType {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Menu,
            1 => Self::Button,
            2 => Self::Api,
            _ => Self::Menu,
        }
    }
}

/// Permission check item
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionCheck {
    pub code: String,
    pub name: String,
    pub permission_type: PermissionType,
}
