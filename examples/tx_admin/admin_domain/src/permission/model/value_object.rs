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

/// PermissionType 转换错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidPermissionTypeError(pub i32);

impl std::fmt::Display for InvalidPermissionTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid permission type value: {}", self.0)
    }
}

impl PermissionType {
    /// 从 i32 安全转换，未知值返回错误
    pub fn try_from_i32(value: i32) -> Result<Self, InvalidPermissionTypeError> {
        match value {
            0 => Ok(Self::Menu),
            1 => Ok(Self::Button),
            2 => Ok(Self::Api),
            _ => Err(InvalidPermissionTypeError(value)),
        }
    }
}

impl From<i32> for PermissionType {
    fn from(value: i32) -> Self {
        Self::try_from_i32(value).unwrap_or(Self::Menu)
    }
}

/// Permission check item
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PermissionCheck {
    pub code: String,
    pub name: String,
    pub permission_type: PermissionType,
}
