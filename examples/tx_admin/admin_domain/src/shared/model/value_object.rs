
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// 删除状态
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DeletedStatus {
    /// 正常状态
    #[default]
    Normal = 0,
    /// 已删除状态
    Deleted = 1,
}

/// 租户ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TenantId(u64);

impl TenantId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }
}
impl Default for TenantId {
    fn default() -> Self {
        Self(0)
    }
}
impl From<u64> for TenantId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<TenantId> for u64 {
    fn from(id: TenantId) -> Self {
        id.0
    }
}

impl FromStr for TenantId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Self)
    }
}

impl schemars::JsonSchema for TenantId {
    fn schema_name() -> String {
        "TenantId".to_string()
    }
    fn json_schema(r#gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        r#gen.subschema_for::<u64>()
    }
}

impl Serialize for TenantId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for TenantId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = u64::deserialize(deserializer)?;
        Ok(TenantId(id))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionEctData {
    /// 租户ID
    pub tenant_id: TenantId,
    /// 用户部门列表
    pub dept_ids: Vec<u64>,
    /// 用户角色列表
    pub role_ids: Vec<u64>,
    /// 登录IP
    pub login_ip: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // ── DeletedStatus ──

    #[test]
    fn test_deleted_status_default_is_normal() {
        assert_eq!(DeletedStatus::default(), DeletedStatus::Normal);
    }

    #[test]
    fn test_deleted_status_equality() {
        assert_eq!(DeletedStatus::Normal, DeletedStatus::Normal);
        assert_eq!(DeletedStatus::Deleted, DeletedStatus::Deleted);
        assert_ne!(DeletedStatus::Normal, DeletedStatus::Deleted);
    }

    #[test]
    fn test_deleted_status_serialize_deserialize() {
        let json = serde_json::to_string(&DeletedStatus::Normal).unwrap();
        assert_eq!(json, "0");
        let json = serde_json::to_string(&DeletedStatus::Deleted).unwrap();
        assert_eq!(json, "1");

        let back: DeletedStatus = serde_json::from_str("0").unwrap();
        assert_eq!(back, DeletedStatus::Normal);
        let back: DeletedStatus = serde_json::from_str("1").unwrap();
        assert_eq!(back, DeletedStatus::Deleted);
    }

    // ── TenantId ──

    #[test]
    fn test_tenant_id_new() {
        let tid = TenantId::new(42);
        assert_eq!(tid.into_inner(), 42);
    }

    #[test]
    fn test_tenant_id_default_is_zero() {
        let tid = TenantId::default();
        assert_eq!(tid.into_inner(), 0);
    }

    #[test]
    fn test_tenant_id_from_u64() {
        let tid = TenantId::from(100u64);
        assert_eq!(tid.into_inner(), 100);
    }

    #[test]
    fn test_tenant_id_into_u64() {
        let tid = TenantId::new(55);
        let val: u64 = tid.into();
        assert_eq!(val, 55);
    }

    #[test]
    fn test_tenant_id_from_str_valid() {
        let tid: TenantId = "123".parse().unwrap();
        assert_eq!(tid.into_inner(), 123);
    }

    #[test]
    fn test_tenant_id_from_str_invalid() {
        let result = TenantId::from_str("not_a_number");
        assert!(result.is_err());
    }

    #[test]
    fn test_tenant_id_from_str_zero() {
        let tid = TenantId::from_str("0").unwrap();
        assert_eq!(tid.into_inner(), 0);
    }

    #[test]
    fn test_tenant_id_equality() {
        assert_eq!(TenantId::new(1), TenantId::new(1));
        assert_ne!(TenantId::new(1), TenantId::new(2));
    }

    #[test]
    fn test_tenant_id_ordering() {
        assert!(TenantId::new(1) < TenantId::new(2));
        assert!(TenantId::new(10) > TenantId::new(5));
    }

    #[test]
    fn test_tenant_id_hash() {
        let mut set = std::collections::HashSet::new();
        set.insert(TenantId::new(1));
        set.insert(TenantId::new(1));
        set.insert(TenantId::new(2));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_tenant_id_serialize_deserialize() {
        let tid = TenantId::new(42);
        let json = serde_json::to_string(&tid).unwrap();
        assert_eq!(json, "42");
        let back: TenantId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TenantId::new(42));
    }

    #[test]
    fn test_tenant_id_clone_copy() {
        let tid = TenantId::new(7);
        let tid2 = tid;
        assert_eq!(tid, tid2); // Copy
        let tid3 = tid.clone();
        assert_eq!(tid, tid3); // Clone
    }

    // ── SessionEctData ──

    #[test]
    fn test_session_ect_data_creation() {
        let data = SessionEctData {
            tenant_id: TenantId::new(1),
            dept_ids: vec![10, 20],
            role_ids: vec![1, 2, 3],
            login_ip: "192.168.1.1".into(),
        };
        assert_eq!(data.tenant_id, TenantId::new(1));
        assert_eq!(data.dept_ids, vec![10, 20]);
        assert_eq!(data.role_ids, vec![1, 2, 3]);
        assert_eq!(data.login_ip, "192.168.1.1");
    }

    #[test]
    fn test_session_ect_data_empty_collections() {
        let data = SessionEctData {
            tenant_id: TenantId::default(),
            dept_ids: vec![],
            role_ids: vec![],
            login_ip: "127.0.0.1".into(),
        };
        assert!(data.dept_ids.is_empty());
        assert!(data.role_ids.is_empty());
    }

    #[test]
    fn test_session_ect_data_serialize_deserialize() {
        let data = SessionEctData {
            tenant_id: TenantId::new(5),
            dept_ids: vec![1],
            role_ids: vec![2, 3],
            login_ip: "10.0.0.1".into(),
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: SessionEctData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tenant_id, TenantId::new(5));
        assert_eq!(back.dept_ids, vec![1]);
        assert_eq!(back.role_ids, vec![2, 3]);
        assert_eq!(back.login_ip, "10.0.0.1");
    }
}
