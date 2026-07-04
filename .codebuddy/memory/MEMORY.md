# tx_di 项目长期记忆

## 项目结构
- Rust 工作区，DI（依赖注入）框架
- `tx-di-macros`：proc-macro crate，提供 `#[derive(Component)]` 宏
- `tx-di-core`：核心运行时 crate（Component trait、Store、App、生命周期、拓扑排序、AOP）
- `common/`：通用工具 crate（tx_common、tx_error 等）
- `plugins/`：插件 crate（如 tx_di_log）
- `examples/`：示例应用（tx_admin 等）

## tx-di-macros 模块结构（2026-07-04 重构后）
原 `comp.rs`(703行单文件) + `utils.rs` 拆分为职责清晰的多模块：
- `attr/` — 属性解析（`comp_attr.rs` 解析 `#[component(...)]`，`field_attr.rs` 解析 `#[tx_cst(...)]`）
- `classify/fields.rs` — 字段分类 `FieldKind` 枚举
- `codegen/` — 代码生成（`mod.rs` 编排 + `CodeGenContext`，`component_impl.rs`、`factory.rs`、`inner_init.rs`、`meta_entry.rs`）
- `type_utils.rs` — 类型检测（Arc/Option/Arc<dyn Trait>）
- `name_utils.rs` — 命名转换（camel_to_snake 等）

数据流：属性解析 → 字段分类 → 构建 CodeGenContext → 各 codegen 子模块生成片段 → 组装

## 已知问题
- `examples/` 中部分 crate 引用 `tx_di_core::tx_comp`（不存在的宏），属预先存在的错误，与 Component derive 宏无关

## 测试
- `cargo test -p tx-di-core` 含 38 个测试覆盖宏全部功能路径，重构后全部通过
