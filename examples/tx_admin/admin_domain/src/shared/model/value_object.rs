
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