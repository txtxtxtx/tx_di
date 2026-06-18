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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // ── PermissionType::try_from_i32 ──

    #[test]
    fn test_permission_type_try_from_i32_valid() {
        assert_eq!(PermissionType::try_from_i32(0).unwrap(), PermissionType::Menu);
        assert_eq!(PermissionType::try_from_i32(1).unwrap(), PermissionType::Button);
        assert_eq!(PermissionType::try_from_i32(2).unwrap(), PermissionType::Api);
    }

    #[test]
    fn test_permission_type_try_from_i32_invalid() {
        let err = PermissionType::try_from_i32(99).unwrap_err();
        assert_eq!(err.0, 99);
        assert_eq!(err.to_string(), "invalid permission type value: 99");
    }

    #[test]
    fn test_permission_type_try_from_i32_negative() {
        let err = PermissionType::try_from_i32(-1).unwrap_err();
        assert_eq!(err.0, -1);
    }

    // ── PermissionType::From<i32> ──

    #[test]
    fn test_permission_type_from_i32_valid() {
        assert_eq!(PermissionType::from(0), PermissionType::Menu);
        assert_eq!(PermissionType::from(1), PermissionType::Button);
        assert_eq!(PermissionType::from(2), PermissionType::Api);
    }

    #[test]
    fn test_permission_type_from_i32_invalid_fallback_to_menu() {
        assert_eq!(PermissionType::from(99), PermissionType::Menu);
        assert_eq!(PermissionType::from(-1), PermissionType::Menu);
    }

    // ── PermissionType serde roundtrip ──

    #[test]
    fn test_permission_type_serialize_deserialize() {
        let pt = PermissionType::Button;
        let json = serde_json::to_string(&pt).unwrap();
        let back: PermissionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PermissionType::Button);
    }

    // ── InvalidPermissionTypeError display ──

    #[test]
    fn test_invalid_permission_type_error_display() {
        let err = InvalidPermissionTypeError(42);
        assert_eq!(err.to_string(), "invalid permission type value: 42");
    }

    #[test]
    fn test_invalid_permission_type_error_debug() {
        let err = InvalidPermissionTypeError(5);
        assert_eq!(format!("{:?}", err), "InvalidPermissionTypeError(5)");
    }

    // ── PermissionCheck ──

    #[test]
    fn test_permission_check_equality() {
        let a = PermissionCheck {
            code: "system:user:view".into(),
            name: "View Users".into(),
            permission_type: PermissionType::Menu,
        };
        let b = PermissionCheck {
            code: "system:user:view".into(),
            name: "View Users".into(),
            permission_type: PermissionType::Menu,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_permission_check_inequality_different_code() {
        let a = PermissionCheck {
            code: "system:user:view".into(),
            name: "View".into(),
            permission_type: PermissionType::Menu,
        };
        let b = PermissionCheck {
            code: "system:user:edit".into(),
            name: "View".into(),
            permission_type: PermissionType::Menu,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_permission_check_hash_in_set() {
        let mut set = std::collections::HashSet::new();
        set.insert(PermissionCheck {
            code: "perm:a".into(),
            name: "A".into(),
            permission_type: PermissionType::Menu,
        });
        set.insert(PermissionCheck {
            code: "perm:a".into(),
            name: "A".into(),
            permission_type: PermissionType::Menu,
        });
        assert_eq!(set.len(), 1); // dedup
    }
}
