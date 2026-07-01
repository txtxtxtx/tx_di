//! tx-di-core 集成测试 — 验证新架构的核心功能

use std::sync::Arc;

use tx_di_core::{Component, DepsTuple, BuildContext};

// ── 测试组件定义 ──────────────────────────────────────────────────────────

/// 无依赖的基础组件
#[derive(Component, Default)]
pub struct DbPool;

/// 依赖 DbPool 的服务
#[derive(Component)]
pub struct UserService {
    pub db: Arc<DbPool>,
}

/// 带自定义值的组件
#[derive(Component, Default)]
pub struct Logger {
    #[tx_cst("info".to_string())]
    pub level: String,
}

/// 带跳过字段的组件
#[derive(Component, Default)]
pub struct Cache {
    #[tx_cst(skip)]
    pub temp: Vec<u8>,
}

// ── 测试用例 ──────────────────────────────────────────────────────────────

#[test]
fn test_basic_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let db = ctx.inject::<DbPool>();
    let _ = db;
}

#[test]
fn test_dependency_inject() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let svc = ctx.inject::<UserService>();
    let _ = svc.db;
}

#[test]
fn test_custom_value() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let logger = ctx.inject::<Logger>();
    assert_eq!(logger.level, "info");
}

#[test]
fn test_skip_field() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let cache = ctx.inject::<Cache>();
    assert!(cache.temp.is_empty());
}

#[test]
fn test_singleton_scope() {
    let ctx = BuildContext::new::<std::path::PathBuf>(None);
    let db1 = ctx.inject::<DbPool>();
    let db2 = ctx.inject::<DbPool>();
    assert!(Arc::ptr_eq(&db1, &db2));
}
