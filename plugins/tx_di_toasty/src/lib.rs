//! tx_di_toasty — 基于 Toasty ORM 的 tx-di 数据库插件
//!
//! 封装 [Toasty](https://github.com/tokio-rs/toasty) 0.6+ 异步 ORM，集成到 tx-di 依赖注入框架，
//! 支持 SQLite / PostgreSQL / MySQL / DynamoDB 多数据库切换。
//!
//! # 快速开始
//!
//! ```toml
//! # Cargo.toml
//! tx_di_toasty = { path = "plugins/tx_di_toasty", features = ["sqlite"] }
//! ```
//!
//! ```toml
//! # config/config.toml
//! [toasty_config]
//! database_url = "sqlite://gb28181.db"
//! # database_url = "postgresql://user:pass@localhost/gb28181"
//! # database_url = "mysql://user:pass@localhost/gb28181"
//! auto_schema = true
//! ```
//!
//! # Feature Flags
//!
//! | Feature      | 数据库       |
//! |-------------|-------------|
//! | `sqlite`    | SQLite（默认）|
//! | `postgresql` | PostgreSQL   |
//! | `mysql`      | MySQL        |
//! | `dynamodb`   | DynamoDB    |

mod config;
pub mod err;
mod plugin;

pub use config::ToastyConfig;
pub use err::ToastyErr;
pub use plugin::{ToastyPlugin, ToastyDb};

/// 事务类型（toasty ORM 原生的数据库事务）
///
/// `Transaction` 实现了 toasty 的 `Executor`，事务内模型查询可直接
/// `xxx.exec(&mut tx)`（与 `&mut Db` 用法一致）。
pub type Transaction<'a> = toasty::db::Transaction<'a>;

/// 便捷分页查询宏：`COUNT` + `LIMIT/OFFSET` 一步完成，返回 `(Vec<Row>, i64 总数)`。
///
/// 封装了分页查询的重复样板（count + limit/offset + 错误映射 + offset 计算），
/// 行 → 领域对象的转换由调用方完成（兼容同步 `to_domain` 与异步 `to_full_domain`）。
///
/// # 为什么用宏而不是泛型函数
///
/// 每个模型生成独立的 `XxxQuery` 类型（非 toasty 统一 trait），且查询需构建两次
/// （count 与 list 各一次，无副作用），宏内联展开两次 `$body` 最简洁可靠。
///
/// # 用法
///
/// ```rust,ignore
/// let (rows, total) = tx_di_toasty::toasty_page!(
///     self.plugin.db().clone(),      // 数据库句柄
///     page,                          // Page<T> 引用（用于计算 offset/size）
///     {                              // 构建查询（含过滤/排序），返回 XxxQuery
///         let mut q = SysUser::all().filter(SysUser::fields().deleted().eq(Deleted::No));
///         if let Some(ref name) = query.name {
///             q = q.filter(SysUser::fields().name().like_with_escape(
///                 format!("%{}%", tx_di_toasty::like_escape(name)), '\\',
///             ));
///         }
///         q
///     },
///     |e| db_err(e, RepositoryError::DatabaseUser)   // 错误转换闭包
/// );
///
/// // 同步转换：
/// let list = rows.iter().map(Self::to_domain).collect::<Vec<_>>();
/// // 或异步转换（关联表懒加载）：
/// let mut list = Vec::new();
/// for r in rows { list.push(self.to_full_domain(&r).await?); }
/// Ok(Page::new(list, page.page, page.size, total))
/// ```
///
/// 要求：必须在返回 `AppResult`（或兼容 `?`）的 async 函数中调用；
/// `$body` 会执行两次（count 与 list），请勿在其中放入有副作用的逻辑。
#[macro_export]
macro_rules! toasty_page {
    ($db:expr, $page:expr, $body:block, $err:expr) => {{
        let mut __db = $db;
        // COUNT（SQL 层）
        let __total = { $body }
            .count()
            .exec(&mut __db)
            .await
            .map_err($err)?;
        // 列表（SQL LIMIT/OFFSET）
        let __offset = $page.offset() as usize;
        let __size = $page.size as usize;
        let __rows = { $body }
            .limit(__size)
            .offset(__offset)
            .exec(&mut __db)
            .await
            .map_err($err)?;
        (__rows, __total as i64)
    }};
}

/// 转义 SQL `LIKE` 模式中的通配符（`%`、`_`）与转义字符本身（`\`）。
///
/// 配合 `like_with_escape(pattern, '\\')` 使用，使用户输入的检索关键字
/// 按**字面值**匹配，避免 `%` / `_` 被当作通配符改变查询语义。
///
/// > 说明：`.like()` / `.ilike()` 是参数化查询（prepared statement），
/// > **没有 SQL 注入风险**；此函数仅处理 LIKE 通配符的语义转义。
///
/// # 示例
/// ```rust,ignore
/// let kw = tx_di_toasty::like_escape(user_input);
/// q.filter(SysXxx::fields().name().like_with_escape(format!("%{kw}%"), '\\'));
/// ```
pub fn like_escape(s: &str) -> String {
    // 先转义反斜杠本身，再转义通配符，避免二次转义
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 便捷事务宏：开启事务 → 执行代码块 → 提交（失败自动回滚）。
///
/// # 为什么需要宏而不是闭包函数
///
/// toasty 的 `Transaction` 借用 `&mut Db` 且实现 `Drop`（drop 时自动回滚），
/// 与"异步闭包借用事务"的 HRTB 生命周期相互冲突（`FnOnce + BoxFuture` 与
/// `AsyncFnOnce` 在 stable Rust 上均无法编译通过）。宏将事务模板**内联展开**
/// 到调用方作用域，借用生命周期由编译器 NLL 正常处理，从而规避该限制。
///
/// # 用法
///
/// ```rust,ignore
/// // 参数 1：数据库句柄（`ToastyPlugin::db().clone()`）
/// // 参数 2：事务变量名（`tx`，代码块内用 `&mut *tx` 执行模型操作）
/// // 参数 3：事务代码块，必须返回 `RIE<T>`
/// tx_di_toasty::toasty_transaction!(self.plugin.db().clone(), tx, {
///     SysUser::create().username("a").exec(&mut *tx).await.map_err(|e| ...)?;
///     SysUserRole::create().user_id(1).role_id(2).exec(&mut *tx).await.map_err(|e| ...)?;
///     Ok(())
/// })
/// ```
///
/// - 代码块必须返回 `RIE<T>`（`Ok(T)` 提交，`Err` 自动回滚）
/// - 代码块内通过 `&mut *tx` 执行模型操作（`tx` 为 `&mut Transaction`）
/// - 必须在 `async` 上下文中调用
///
/// > 注意：由于 `macro_rules!` 的标识符卫生性，宏内无法"隐式"暴露变量给代码块，
/// > 因此事务变量名需由调用方显式传入（参数 2），以保证卫生上下文一致。
#[macro_export]
macro_rules! toasty_transaction {
    ($db:expr, $tx:ident, $body:block) => {{
        let mut __db = $db;
        let mut __tx = __db.transaction().await.map_err(|e| {
            tx_di_core::AppError::with_context(
                $crate::ToastyErr::TxBeginFailed,
                format!("事务开启失败: {e}"),
            )
        })?;

        // 独立作用域：async 块借用 `$tx`，await 结束后借用释放，再消费 `__tx`
        let __result = {
            let $tx = &mut __tx;
            async { $body }.await
        };

        match __result {
            Ok(__v) => {
                __tx.commit().await.map_err(|e| {
                    tx_di_core::AppError::with_context(
                        $crate::ToastyErr::TxCommitFailed,
                        format!("事务提交失败: {e}"),
                    )
                })?;
                Ok(__v)
            }
            Err(__e) => Err(__e), // 未 commit，`__tx` drop 时自动回滚
        }
    }};
}