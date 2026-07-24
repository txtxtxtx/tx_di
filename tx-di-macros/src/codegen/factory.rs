//! 生成 factory 闭包
//!
//! 根据是否为配置组件生成两条不同的工厂路径：
//! - 配置组件：从 `AppAllConfig` 反序列化
//! - 普通组件：`Deps::resolve` → `build` → 注入必选 trait → `inner_init`

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::codegen::CodeGenContext;
use crate::name_utils::camel_to_snake;

/// 生成 factory 闭包 TokenStream
pub fn gen_factory_fn(ctx: &CodeGenContext) -> TokenStream2 {
    let struct_name = &ctx.struct_name;

    if ctx.comp_attr.is_config_component() {
        // ── 配置组件：从配置反序列化 ──────────────────────────────────
        let config_key = if let Some(Some(custom_key)) = &ctx.comp_attr.conf {
            quote! { #custom_key }
        } else {
            let snake_name = camel_to_snake(&struct_name.to_string());
            quote! { #snake_name }
        };

        quote! {
            |store: &::tx_di_core::Store| {
                let app_config = ::tx_di_core::inject_from_store::<::tx_di_core::AppAllConfig>(store);
                let config_key = #config_key;
                let mut config = if let Some(value) = app_config.get_value(config_key) {
                    <#struct_name as serde::Deserialize>::deserialize(value.clone())
                        .unwrap_or_else(|e| {
                            panic!(
                                "[di] 配置组件 '{}' 反序列化失败 (key='{}'): {}\n\
                                 请检查配置文件中该字段的类型和格式是否正确。",
                                stringify!(#struct_name), config_key, e
                            )
                        })
                } else {
                    let empty_table = ::tx_di_core::Value::Table(::tx_di_core::map::Map::new());
                    <#struct_name as serde::Deserialize>::deserialize(empty_table)
                        .unwrap_or_else(|e| {
                            panic!(
                                "[di] 配置组件 '{}' 缺少配置 key='{}', 且默认值反序列化也失败: {}\n\
                                 请在配置文件中添加该 section, 或为所有字段提供 #[serde(default)]。",
                                stringify!(#struct_name), config_key, e
                            )
                        })
                };
                ::tracing::debug!("{} build 成功", stringify!(#struct_name));
                Box::new(config) as Box<dyn std::any::Any + Send + Sync>
            }
        }
    } else {
        // ── 普通组件：resolve → build(store) → inner_init ────────────
        // build 接收 store 以直接注入 trait object 依赖（无需 unsafe）。
        quote! {
            |store: &::tx_di_core::Store| {
                let deps = <#struct_name as ::tx_di_core::Component>::Deps::resolve(store)
                    .unwrap_or_else(|e| panic!("{}", e));
                let mut instance = <#struct_name as ::tx_di_core::Component>::build(deps, store);
                if let Err(e) = <#struct_name as ::tx_di_core::Component>::inner_init(&mut instance, store) {
                    panic!("[di] 组件 '{}' inner_init 失败: {}", stringify!(#struct_name), e);
                }
                ::tracing::debug!("{} build 成功", stringify!(#struct_name));
                Box::new(instance) as Box<dyn std::any::Any + Send + Sync>
            }
        }
    }
}
