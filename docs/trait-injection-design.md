# Trait Object 注入方案

## 问题

当前框架的容器 key 是具体类型的 `TypeId`，但用户希望注入 `Arc<dyn Trait>`：

```rust
// 注册时：key = TypeId::of::<B>()
registry.insert(TypeId::of::<B>(), factory);

// 注入时：想要 Arc<dyn A>
let a: Arc<dyn A> = inject::<dyn A>();  // TypeId::of::<dyn A>() ≠ TypeId::of::<B>() → 找不到
```

## 解决方案：TraitWrapper 包装

### 核心思路

用 `TraitWrapper<dyn Trait>` 包装 `Arc<dyn Trait>`，使其可以存储在 `Box<dyn Any>` 中：

```rust
#[repr(transparent)]
pub struct TraitWrapper<T: ?Sized> {
    pub inner: Arc<T>,
}

// TraitWrapper<dyn Trait> 自动实现 Any + Send + Sync
// 可以存储在 DashMap<TypeId, CompRef> 中
```

### 验证

```rust
use std::any::{Any, TypeId};
use std::sync::Arc;

trait A: Any + Send + Sync {
    fn a(&self) -> String;
}

struct B { name: String }
impl A for B {
    fn a(&self) -> String { self.name.clone() }
}

#[repr(transparent)]
struct TraitWrapper<T: ?Sized> {
    inner: Arc<T>,
}

fn main() {
    let b = B { name: "hello".to_string() };
    let a: Arc<dyn A> = Arc::new(b);
    
    // 存储
    let wrapper = TraitWrapper { inner: a };
    let stored: Box<dyn Any + Send + Sync> = Box::new(wrapper);
    
    // 注入
    let result = stored.downcast_ref::<TraitWrapper<dyn A>>();
    let injected: Arc<dyn A> = result.unwrap().inner.clone();
    
    println!("{}", injected.a()); // 输出: hello
}
```

---

## 完整改动清单

### 1. 新增 TraitWrapper 类型

**文件**: `tx-di-core/src/di/comp/mod.rs` 或新建 `trait_wrapper.rs`

```rust
/// Trait Object 包装器
///
/// 用于将 `Arc<dyn Trait>` 存储在类型擦除的容器中。
/// 由于 `Arc<dyn Trait>` 本身不实现 `Any`，需要用此包装器中转。
///
/// # 要求
///
/// 被包装的 trait 必须继承 `Any + Send + Sync`：
/// ```rust
/// trait MyTrait: Any + Send + Sync { ... }
/// ```
#[repr(transparent)]
pub struct TraitWrapper<T: ?Sized> {
    pub inner: Arc<T>,
}

// TraitWrapper<dyn Trait> 自动派生 Any + Send + Sync
// 因为它的内存布局与 Arc<dyn Trait> 相同
```

### 2. ComponentMeta 增加 aliases 字段

**文件**: `tx-di-core/src/di/comp/mod.rs`

```rust
/// Trait 别名描述
pub struct TraitAlias {
    /// trait 的 TypeId（运行时）
    pub trait_type_id: fn() -> TypeId,
    /// 工厂函数：构建具体类型并包装为 TraitWrapper<dyn Trait>
    pub wrapper_factory: StoreFactoryFn,
}

pub struct ComponentMeta {
    pub type_id: fn() -> TypeId,
    pub name: &'static str,
    pub scope: Scope,
    pub deps: &'static [fn() -> TypeId],
    pub factory_fn: Option<StoreFactoryFn>,
    
    /// 新增：trait 别名列表
    pub aliases: &'static [TraitAlias],
    
    pub init_sort_fn: fn() -> i32,
    pub init_fn: Option<fn(Arc<App>, CancellationToken) -> RIE<()>>,
    pub async_init_fn: Option<fn(Arc<App>, CancellationToken) -> BoxFuture>,
    pub async_run_fn: Option<fn(Arc<App>, CancellationToken) -> BoxFuture>,
}
```

### 3. 宏支持 `as_trait` 属性

**文件**: `tx-di-macros/src/comp.rs`

```rust
// 用户写
#[tx_comp(as_trait = "UserRepository")]
pub struct SqliteUserRepository { ... }

// 或多个 trait
#[tx_comp(as_trait = ["UserRepository", "dyn Repository<User>"])]
pub struct SqliteUserRepository { ... }
```

宏解析逻辑：

```rust
fn parse_component_attr(attr_tokens: TokenStream) -> SynResult<CompAttr> {
    // ... 现有逻辑 ...
    
    // 新增：解析 as_trait 参数
    if key == "as_trait" {
        if input.peek(Token![=]) {
            let _eq: Token![=] = input.parse()?;
            
            // 支持字符串或数组
            if input.peek(syn::token::Bracket) {
                // 数组形式: as_trait = ["Trait1", "Trait2"]
                let content;
                syn::bracketed!(content in input);
                let traits: Vec<String> = content.parse_terminated(
                    |input| {
                        let lit: syn::LitStr = input.parse()?;
                        Ok(lit.value())
                    },
                    Token![,]
                )?.into_iter().collect();
                aliases = traits;
            } else {
                // 单个形式: as_trait = "Trait"
                let lit: syn::LitStr = input.parse()?;
                aliases = vec![lit.value()];
            }
        }
    }
}
```

宏代码生成：

```rust
// 为每个 alias 生成 TraitAlias
let trait_aliases: Vec<TokenStream2> = comp_attr.aliases.iter().map(|alias| {
    let trait_type: Type = syn::parse_str(alias).unwrap();
    quote! {
        ::tx_di_core::TraitAlias {
            trait_type_id: || ::std::any::TypeId::of::<dyn #trait_type>(),
            wrapper_factory: |store| {
                let instance = <#struct_name as ::tx_di_core::ComponentDescriptor>::build(store);
                let arc: ::std::sync::Arc<dyn #trait_type> = ::std::sync::Arc::new(instance);
                let wrapper = ::tx_di_core::TraitWrapper { inner: arc };
                ::std::boxed::Box::new(wrapper) as ::std::boxed::Box<dyn ::std::any::Any + Send + Sync>
            },
        }
    }
}).collect();

// ComponentMeta 中
ComponentMeta {
    type_id: || TypeId::of::<#struct_name>(),
    aliases: &[ #( #trait_aliases ),* ],
    // ... 其他字段
}
```

### 4. 注册逻辑修改

**文件**: `tx-di-core/src/di/mod.rs`

```rust
impl BuildContext {
    fn auto_register_all(&mut self) {
        let metas: Vec<&ComponentMeta> = COMPONENT_REGISTRY.iter().collect();
        let sorted_ids = topo_sort(&metas);
        
        for tid in &sorted_ids {
            if let Some(meta) = metas.iter().find(|m| (m.type_id)() == *tid) {
                // 注册具体类型
                if let Some(factory_fn) = meta.factory_fn {
                    self.register_factory_boxed((meta.type_id)(), meta.scope, factory_fn);
                }
                
                // 注册 trait 别名
                for alias in meta.aliases {
                    let trait_tid = (alias.trait_type_id)();
                    let wrapper_instance = (alias.wrapper_factory)(&self.store);
                    
                    // 根据 scope 决定存储方式
                    match meta.scope {
                        Scope::Singleton => {
                            let arc = Arc::from(wrapper_instance);
                            self.store.insert(trait_tid, CompRef::Cached(arc));
                        }
                        Scope::Prototype => {
                            let factory = alias.wrapper_factory;
                            let closure = move |store: &DashMap<TypeId, CompRef>| -> Arc<dyn Any + Send + Sync> {
                                let boxed = factory(store);
                                Arc::from(boxed)
                            };
                            self.store.insert(trait_tid, CompRef::Factory(Arc::new(closure)));
                        }
                    }
                }
                
                self.metas.push(meta);
            }
        }
    }
}
```

### 5. 注入函数修改

**文件**: `tx-di-core/src/di/comp/comp_ref.rs`

```rust
/// 从 store 注入具体类型
pub fn inject_from_store<T: Any + Send + Sync + 'static>(
    store: &DashMap<TypeId, CompRef>,
) -> Arc<T> {
    let tid = TypeId::of::<T>();
    // ... 现有逻辑不变
}

/// 从 store 注入 trait object（新增）
pub fn inject_trait_from_store<T: ?Sized + Any + Send + Sync + 'static>(
    store: &DashMap<TypeId, CompRef>,
) -> Arc<T> {
    let tid = TypeId::of::<TraitWrapper<T>>();
    store
        .get(&tid)
        .map(|entry| match &*entry {
            CompRef::Cached(any_arc) => any_arc.clone(),
            CompRef::Factory(f) => f(store),
        })
        .and_then(|any_arc| any_arc.downcast_ref::<TraitWrapper<T>>())
        .map(|wrapper| wrapper.inner.clone())
        .unwrap_or_else(|| {
            panic!(
                "[di] 注入失败: trait `{}` 未注册。\n\
                 请确认:\n\
                 1. 实现该 trait 的结构体已标注 #[tx_comp(as_trait = \"...\")]\n\
                 2. trait 继承了 Any + Send + Sync",
                std::any::type_name::<T>()
            )
        })
}
```

### 6. 宏注入代码生成修改

**文件**: `tx-di-macros/src/comp.rs`

```rust
// 修改 build_fields 生成逻辑
FieldKind::Inject { ty } => {
    let inject_ty = strip_arc(ty);
    
    // 判断是否是 trait object（包含 `dyn` 关键字）
    let type_str = quote!(#inject_ty).to_string();
    if type_str.starts_with("dyn ") {
        // trait object 注入
        quote! { #fname: ::tx_di_core::inject_trait_from_store::<#inject_ty>(store) }
    } else {
        // 具体类型注入
        quote! { #fname: ::tx_di_core::inject_from_store::<#inject_ty>(store) }
    }
}

// 修改 DEP_IDS 生成逻辑
FieldKind::Inject { ty } => {
    let inject_ty = strip_arc(ty);
    let type_str = quote!(#inject_ty).to_string();
    if type_str.starts_with("dyn ") {
        // trait object 依赖
        Some(quote! { || ::std::any::TypeId::of::<::tx_di_core::TraitWrapper<#inject_ty>>() })
    } else {
        // 具体类型依赖
        Some(quote! { || ::std::any::TypeId::of::<#inject_ty>() })
    }
}
```

### 7. 导出新类型

**文件**: `tx-di-core/src/lib.rs`

```rust
pub use di::comp::TraitWrapper;
pub use di::comp::TraitAlias;
pub use di::comp::comp_ref::inject_trait_from_store;
```

---

## 用户使用示例

### 定义 trait

```rust
// trait 必须继承 Any + Send + Sync
pub trait UserRepository: Any + Send + Sync {
    fn find_by_id(&self, id: u64) -> AppResult<User>;
    fn find_by_username(&self, username: &str) -> AppResult<Option<User>>;
    fn insert(&self, user: &User) -> AppResult<()>;
}
```

### 实现并注册

```rust
#[tx_comp(as_trait = "UserRepository")]
pub struct SqliteUserRepository {
    pool: Arc<DbPool>,
}

impl UserRepository for SqliteUserRepository {
    fn find_by_id(&self, id: u64) -> AppResult<User> { ... }
    // ...
}
```

### 注入使用

```rust
pub struct UserService {
    repo: Arc<dyn UserRepository>,  // 宏自动处理注入
}
```

### 测试时替换

```rust
#[cfg(test)]
pub struct MockUserRepository { ... }

#[cfg(test)]
impl UserRepository for MockUserRepository { ... }

// 测试 setup
fn setup_test_app() -> BuildContext {
    let mut ctx = BuildContext::new(None);
    
    // 手动注册 mock
    let mock = MockUserRepository::new();
    let wrapper = TraitWrapper { inner: Arc::new(mock) };
    ctx.store.insert(
        TypeId::of::<TraitWrapper<dyn UserRepository>>(),
        CompRef::Cached(Arc::new(wrapper)),
    );
    
    ctx
}
```

---

## 注意事项

1. **trait 约束**：被注入的 trait 必须继承 `Any + Send + Sync`
2. **类型判断**：宏通过字符串是否以 `dyn ` 开头来判断是否是 trait object
3. **TypeId 唯一性**：`TraitWrapper<dyn A>` 和 `TraitWrapper<dyn B>` 的 TypeId 不同，不会冲突
4. **性能**：注入时有一次 `downcast_ref` 调用，开销极小（~1ns）
5. **兼容性**：不影响现有具体类型的注入逻辑

---

## 改动影响

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `tx-di-core/src/lib.rs` | 新增导出 | TraitWrapper, TraitAlias, inject_trait_from_store |
| `tx-di-core/src/di/comp/mod.rs` | 新增类型 | TraitWrapper, TraitAlias |
| `tx-di-core/src/di/comp/comp_ref.rs` | 新增函数 | inject_trait_from_store |
| `tx-di-core/src/di/mod.rs` | 修改逻辑 | auto_register_all 增加 alias 注册 |
| `tx-di-macros/src/comp.rs` | 修改逻辑 | 解析 as_trait，生成 alias 代码 |
| `tx-di-macros/src/utils.rs` | 可能修改 | 增加辅助函数 |

---

## 向后兼容

- 不影响现有代码（没有 `as_trait` 的组件行为不变）
- `inject_from_store` 签名不变
- `ComponentMeta` 新增字段，旧代码使用默认值 `&[]`
