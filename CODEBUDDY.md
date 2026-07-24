# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## Common Commands

### Build & Test

```bash
# Build the entire workspace
cargo build

# Run all tests (core + macros)
cargo test -p tx-di-core -p tx-di-macros

# Run only core tests (serial, to avoid global-state races in integration tests)
cargo test -p tx-di-core -- --test-threads=1

# Run a specific test
cargo test -p tx-di-core --test test_component test_trait_object_inject

# Check compilation only (faster than build)
cargo check -p tx-di-core -p tx-di-macros

# Clippy
cargo clippy -p tx-di-core -p tx-di-macros

# Format
cargo fmt
```

### Running Examples

```bash
# Most examples use TOML configs from configs/ directory
cargo run -p gb_cams          # GB28181 camera simulator
cargo run -p admin_api        # Admin backend API
cargo run -p dfbja7_transformer  # A7→MQTT transformer
```

### Key Dependencies

- `linkme` — compile-time component registration via custom link sections (ELF/Mach-O/PE). Components are collected into a distributed slice `COMPONENT_REGISTRY`.
- `DashMap` — lock-free concurrent HashMap for the component store.
- `tokio` — async runtime for lifecycle hooks (`async_init`, `async_run`, `waiting_exit`).

## Architecture

### Two-Crate Design

The DI framework is split into two interdependent crates:

1. **`tx-di-core`** — Runtime: `Component` trait, `Store` (DashMap-backed type registry), `App` (lifecycle orchestrator), topological sort, AOP interceptors, configuration loading.
2. **`tx-di-macros`** — Proc-macro: `#[derive(Component)]` generates trait impls + linkme registration entries. `#[intercept]` wraps methods with interceptor logic.

### Registration Flow

```
Compile-time:     #[derive(Component)] → codegen/meta_entry.rs →
                  linkme::distributed_slice COMPONENT_REGISTRY

Runtime:          BuildContext::new() → auto_register_all() →
                  topo_sort(metas, trait_impls) → register_factory(meta)
                     ↕ for each component in sorted order
                  build(deps, store) → inner_init → (init → async_init → …)
```

### Store (Type Registry)

`Store` is a flat `DashMap<TypeId, CompRef>` where `CompRef` is either:
- `Cached(Arc<dyn Any>)` — for Singletons
- `Factory(Arc<Fn(&Store) -> Arc<dyn Any>)>` — for Prototypes (new instance per injection)

A separate `trait_impls: DashMap<TypeId, Vec<TraitImplEntry>>` maps trait `TypeId`s to their concrete implementations, enabling `Arc<dyn Trait>` injection.

### `#[derive(Component)]` Code Generation Pipeline

```
attr/comp_attr.rs     →  parse #[component(scope, init, conf, as_trait, intercept, ...)]
attr/field_attr.rs    →  parse #[tx_cst(expr)] / #[tx_cst(skip)]
classify/fields.rs    →  classify each field as Inject | TraitInject | TraitInjectRequired |
                          TraitInjectList | OptionalInject | Optional | Custom | Skip
codegen/mod.rs        →  CodeGenContext orchestrates all sub-generators
  ├── component_impl.rs  →  impl Component { type Deps, build(), inner_init(), lifecycle }
  ├── factory.rs         →  |store| { resolve deps → build → inject traits → inner_init }
  ├── meta_entry.rs      →  linkme-registered ComponentMeta static
  ├── lifecycle.rs       →  lifecycle hook overrides (init/async_init/async_run/shutdown)
  ├── intercept.rs       →  interceptor static + __get_chain() + init override
  └── inner_init.rs      →  trait injection for Option<Arc<dyn Trait>> and Vec<Arc<dyn Trait>>
```

**Field classification order** in `classify/fields.rs`:
1. `#[tx_cst(skip)]` → Skip
2. `#[tx_cst(expr)]` → Custom (user intent always wins)
3. `Arc<dyn Trait>` → TraitInjectRequired (injected in build via `inject_trait_from_store`)
4. `Vec<Arc<dyn Trait>>` → TraitInjectList
5. `Option<Arc<dyn Trait>>` → TraitInject (injected in inner_init via `try_inject_trait_from_store`)
6. `Option<Arc<T>>` → OptionalInject
7. `Option<T>` → Optional
8. `Arc<T>` / others → Inject

### Lifecycle Phases

| Phase | When | Description |
|-------|------|-------------|
| `build` | Factory call | Construct struct from resolved `Deps` tuple + inject required trait deps |
| `inner_init` | Factory call (right after build) | Inject optional/list trait deps; call user's `init` if `#[component(init)]` |
| `init` (`app_init`) | `App::init()` | Synchronous post-construction setup (interceptor chain setup, etc.) |
| `async_init` | `App::async_init()` | Serial async setup (DB connections, service registration) |
| `async_run` | `App::comp_run()` | Concurrent background tasks (event loops, message consumers) via `tokio::spawn` |
| `shutdown` | `App::shutdown()` | Reverse-topological-order graceful shutdown (idempotent via `AtomicBool`) |

### AOP (Interceptors)

Interceptors store their chain in a **type-level `OnceLock<Arc<InterceptorChain>>`** generated per component type by `intercept.rs`. The `#[intercept]` macro on methods accesses it via `__get_chain()` (a module-level generated function). No global table, no lock, no ABA.

```rust
// Macro generates:
static __INTERCEPTOR_CHAIN: OnceLock<Arc<InterceptorChain>> = OnceLock::new();
fn __get_chain() -> &Arc<InterceptorChain> { ... }

// #[intercept] method calls:
let __chain = __get_chain();
// before → body → after (or around_all for advanced cases)
```

`Interceptor` trait: `before()`, `around(BoxCall)`, `after()`.

### Configuration

`AppAllConfig` stores both `toml_value` and `config_path` (no global static). Configuration components use `#[component(conf)]` and are deserialized from TOML by the factory function. The `get()` / `get_strict()` methods on `AppAllConfig` support strict (return `RIE`) and lenient (return `Option`) access.

### Plugin System

Plugins are crates in `plugins/` that contain `#[derive(Component)]` structs. They register via `linkme` — **must be `use`d explicitly** in the consuming crate, otherwise the linker optimizes them away silently. Each plugin has a `README.md` documenting configuration keys, public components, and API.

Plugin initialization order is controlled by `init_sort`:
- `i32::MIN`: `tx_di_log` (tracing subscriber, must be first)
- `i32::MIN+1`: Config components
- `i32::MIN+2`: `tx_di_toasty`
- `i32::MIN+3`: `tx_di_file`
- `10000`: Default, `tx_di_sip`, etc.
- `i32::MAX`: `tx_di_axum` (web server, starts last)

### Error Handling

`RIE<T>` = `Result<T, AppError>` where `AppError` comes from `tx_error` crate. DI-specific errors use `DiErr` enum codes (RegistryError, AsyncInitError, TaskPanic, InjectError, ConfigError, TraitInjectError). The `auto_register_all` → `BuildContext::new` → `config::new` chain propagates errors via `?` instead of panicking.

### Key Constraints

- Components must be `Send + Sync + 'static` (stored in `Arc<dyn Any + Send + Sync>`)
- Config components need `Deserialize + Default`
- Trait objects for injection need `Trait: Any + Send + Sync`
- Max 16 `Arc<T>` dependencies (DepsTuple compile-time limit)
- Plugin crates need explicit `use plugin_crate;` for linkme registration
- Integration tests share a single binary → global statics can leak between tests when run in parallel; use `--test-threads=1`
