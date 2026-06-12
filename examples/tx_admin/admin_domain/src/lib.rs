pub mod shared;
pub mod user;
pub mod permission;
pub mod role;
pub mod menu;
pub mod department;
pub mod log;
pub mod file;
pub mod dictionary;
pub mod config;
pub mod password;

/// 重新导出 AggregrateRoot 派生宏，方便 crate 内使用 `use crate::AggregateRoot;`
pub use admin_macros::AggregateRoot;

#[cfg(test)]
mod tests {
    use super::*;

}
