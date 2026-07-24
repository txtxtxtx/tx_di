# tx_di 项目长期记忆

## 项目结构
- Rust 工作区 DI 框架：`tx-di-macros`（proc-macro，`#[derive(Component)]`）+ `tx-di-core`（运行时：Component trait、Store、App、生命周期、拓扑排序、AOP）
- `common/` 通用工具；`plugins/` 插件（log/cache/axum/job/sa_token/sip/gb28181/gb_dev/file/toasty/registry）；`examples/` 示例（tx_admin、tx_di_can 等）

## tx-di-macros 结构（2026-07-04 重构后）
- `attr/`（comp_attr/field_attr 解析）→ `classify/fields.rs`（FieldKind）→ `codegen/`（CodeGenContext 编排 + component_impl/factory/inner_init/lifecycle/meta_entry）
- `type_utils.rs` 类型检测；`name_utils.rs` 命名转换

## 框架 API 要点（2026-07-05 迁移后）
- `#[derive(Component)] #[component(scope/init/app_init/app_async_init/app_async_run/shutdown/init_sort/conf/as_trait)]`
- 回调为模块级自由函数，与属性同名：
  - `init(this: &mut T, store: &Store) -> RIE<()>`（覆写 inner_init）
  - `app_init(comp: Arc<T>, app: &Arc<App>) -> RIE<()>`
  - `app_async_init(comp: Arc<T>, app: Arc<App>) -> RIE<()>`（async fn）
  - `app_async_run(comp: Arc<T>, app: Arc<App>, token: CancellationToken) -> RIE<()>`
  - `shutdown(_comp: &T)`（非 &self）
- 字段：`Arc<T>` 注入、`Arc<dyn Trait>` 必选 trait 注入（mem::zeroed 占位 + ptr::write）、`Option<Arc<dyn Trait>>` 可选 trait、`#[tx_cst(expr/skip)]`
- 注意：使用方需 `use tx_di_core::DepsTuple`；异步回调 app 参数须 `Arc<App>`（'static）
- 插件必须显式 `use tx_di_xxx;` 触发 linkme 注册
- `ins_run()` 返回前已完成 init + async_init
- 测试：`cargo test -p tx-di-core`（64+ 测试）

## rsipstack 0.5.x 要点
- 仅 `Via/From/To/CSeq` 在 `rsipstack::sip::typed`；其余头（CallId/Expires/MaxForwards 等）在 `rsipstack::sip` 根
- `Transaction`：Send+Sync 但 !Clone；reply/send 均 `&mut self` async；有 Drop（必须保证最终 reply）
- `HeadersExt` trait 提供 from_header/expires_header 等

## tx_di_sip / gb28181 架构决策
- 用户明确要求**强绑定 rsipstack**，不做解耦 trait 抽象
- `SipTx` 信封：`Arc<Mutex<Option<Transaction>>>` + 缓存 Request + replied 幂等标志 + fake() 测试模式
- `SipMiddleware` trait 经 `as_trait = dyn SipMiddleware` DI 收集，build_chain 洋葱模型；dispatch 兜底 405
- gb28181 认证在 `auth.rs` 的 `Gb28181AuthMiddleware`（sort=10 最外层）；NonceStore 随中间件常驻
- 真 BYE：`SessionInfo` 持 `ClientInviteDialog`（Clone、无 Debug，手写 Debug 跳过）

## tx_di_can（examples/，2026-07-08 完成）
- 无硬件 SimBus 联调：描述库 db/、sim_ecu/ UDS 仿真、hex/flash/record/dbc、XCP+A2L、审计报表、CSV 离线分析、i18n+工程管理
- 89 测试全绿；前端 vue-tsc 通过；待办：产线权限分级、自动化脚本、CCP

## 已知问题
- `examples/` 部分 crate 引用不存在的 `tx_di_core::tx_comp` 宏（预先存在错误）
