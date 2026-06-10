pub mod shared;
pub mod user;
pub mod permission;
pub mod role;
pub mod menu;
pub mod department;

/// 重新导出 AggregrateRoot 派生宏，方便 crate 内使用 `use crate::AggregateRoot;`
pub use admin_macros::AggregateRoot;

#[cfg(test)]
mod tests {
    use super::*;

}
