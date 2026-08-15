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

## tx_di_nacos 插件（2026-08-10 核实）
- **非组件 crate**：不导出 `#[derive(Component)]`，不注册 DI；基于 nacos-sdk 0.8，由应用入口以函数/宏使用
- 能力：配置中心（启动拉取远程配置，TOML 深合并远程覆盖本地）、配置变更优雅重启（`App::graceful_shutdown` 进程不退出）、服务注册（SDK 自动心跳）
- `app_loop! { config = ..., startup = |app| ... }` 宏替代 `BuildContext::new + ins_run + waiting_exit`
- 端点注册：插件侧 `register_endpoints(provider)`（HTTP/gRPC 插件在 app_async_init 调用）→ 宏启动后 `take_endpoints` 收集注册到 Nacos
- 配置节 `[registry_config]`：enabled/nacos_addr/namespace/service_name/auto_register/username/password/config_data_id
- 行为：enabled=false 退化为本地启动；Nacos 不可达降级本地启动 warn 不阻塞；新配置启动失败由进程管理器拉起
- 注册 IP：容器需 `SERVICE_IP` 环境变量（默认 127.0.0.1 仅单机）

## tx_admin 架构（2026-08-15，DDD 战略重构完成并全量测试通过）
- 四层 DDD：admin_api（HTTP axum + gRPC tonic）/ admin_app（AppService + EventBus）/ admin_domain / admin_infra（toasty Repository）
- admin_domain 战略重组为四大边界：`identity`（user/role/menu/department/auth）、`system`（config/dictionary/log/file）、`job`（新增域：Job/JobLog 聚合 + JobRepository trait + JobService）、`shared`（Event trait / Entity / AggregateRoot<E> / AuditFields / DomainEventPublisher / security::password）；顶层 `password` 为向后兼容 re-export
- 领域层仅依赖 admin_macros/tx_error/tx_common，仓储 trait 在领域层、实现在 infra（依赖倒置）
- 强类型事件：删除巨型 DomainEvent 枚举，各域定义独立事件枚举（UserEvent 等）实现 `Event` trait（Any+Send+Sync+Clone）；`DomainEventPublisher::publish(Vec<Arc<dyn Event>>)`，EventBus 按 TypeId 路由 + downcast 分发
- 错误拆分：删除巨型 RepositoryError，各域定义自己的 RepositoryError（`#[err("REPOSITORY")]` 前缀不变，错误码数值不变防前端回归）
- `AggregateRoot` 宏改为 `#[aggregate_root(event = path)]` 属性指定事件类型，聚合根字段 `events: Vec<E>`
- auth 归入 identity 并改为依赖 `UserRepository`（而非跨域 UserService）
- DI 生命周期：build → inner_init → init → async_init → comp_run
- 密码 Argon2id（shared/security/password）；用户/Job 分页均 SQL 层
- 启动：main.rs 用 `tx_di_nacos::app_loop!`
- 测试全绿（2026-08-15）：admin_domain 382、admin_macros 2、admin_app 多文件、admin_api 57 均通过
- 注意：admin_app/admin_infra/admin_api 的**源码目录未移动**（仍在 user/role/... 子目录），仅 admin_domain 内部重组；测试中跨 crate 引用统一用 `admin_domain::identity::*` / `admin_domain::system::*`

## 已知问题
- `examples/` 部分 crate 引用不存在的 `tx_di_core::tx_comp` 宏（预先存在错误）
- `examples/tx_admin/PROJECT.md` 的"当前问题与待优化清单"已部分过时（P2 密码、M2/L7 分页均已修复），以代码为准
