use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// 为聚合根自动实现 `Entity` 和 `AggregateRoot` trait。
///
/// 要求结构体必须包含：
/// - `id` 字段（ID 类型将作为 `Entity::Id`）
/// - `events: Vec<DomainEvent>` 字段
///
/// trait 路径固定为 `crate::shared::model`（由调用方 crate 的上下文解析）。
///
/// # 示例
///
/// ```ignore
/// use crate::AggregateRoot;
///
/// #[derive(Debug, Clone, AggregateRoot)]
/// pub struct User {
///     pub id: u64,
///     pub name: String,
///     events: Vec<DomainEvent>,
/// }
/// ```
#[proc_macro_derive(AggregateRoot)]
pub fn derive_aggregate_root(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_aggregate_root(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_aggregate_root(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    // 遍历结构体字段，提取 id 类型和确认 events 字段存在
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "AggregateRoot only supports named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "AggregateRoot only supports structs",
            ))
        }
    };

    let mut id_type: Option<&syn::Type> = None;
    let mut has_events = false;

    for field in fields {
        let field_name = field.ident.as_ref().map(|i| i.to_string());
        match field_name.as_deref() {
            Some("id") => {
                id_type = Some(&field.ty);
            }
            Some("events") => {
                has_events = true;
            }
            _ => {}
        }
    }

    let id_type = id_type.ok_or_else(|| {
        syn::Error::new_spanned(&input, "missing `id` field in aggregate root")
    })?;

    if !has_events {
        return Err(syn::Error::new_spanned(
            &input,
            "missing `events: Vec<DomainEvent>` field in aggregate root",
        ));
    }

    // trait 路径固定为 crate::shared::model，crate:: 在调用方上下文中解析
    let expanded = quote! {
        impl crate::shared::model::Entity for #name {
            type Id = #id_type;
            fn id(&self) -> Self::Id {
                self.id
            }
        }

        impl crate::shared::model::AggregateRoot for #name {
            fn events(&self) -> &[crate::shared::model::DomainEvent] {
                &self.events
            }
            fn clear_events(&mut self) {
                self.events.clear();
            }
            fn add_event(&mut self, event: crate::shared::model::DomainEvent) {
                self.events.push(event);
            }
        }
    };

    Ok(expanded)
}
