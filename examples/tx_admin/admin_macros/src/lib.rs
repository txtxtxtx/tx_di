use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// 为聚合根自动实现 `Entity` 和 `AggregateRoot` trait。
///
/// 要求结构体必须包含：
/// - `id` 字段（ID 类型将作为 `Entity::Id`）
/// - `events: Vec<DomainEvent>` 字段
///
/// 默认从 `crate::shared::model` 导入 trait 和类型，
/// 可通过 `#[aggregate_root(path = "...")]` 自定义路径。
///
/// # 示例
///
/// ```ignore
/// use admin_macros::AggregateRoot;
///
/// #[derive(Debug, Clone, AggregateRoot)]
/// pub struct User {
///     pub id: u64,
///     pub name: String,
///     events: Vec<DomainEvent>,
/// }
/// ```
///
/// # 自定义路径
///
/// ```ignore
/// #[derive(AggregateRoot)]
/// #[aggregate_root(path = "custom_path::model")]
/// pub struct User { ... }
/// ```
#[proc_macro_derive(AggregateRoot, attributes(aggregate_root))]
pub fn derive_aggregate_root(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_aggregate_root(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn expand_aggregate_root(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    // 解析 #[aggregate_root(path = "...")] 属性
    let mut domain_path: syn::Path = syn::parse_quote!(crate::shared::model);

    for attr in &input.attrs {
        if attr.path().is_ident("aggregate_root") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("path") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    domain_path = value.parse()?;
                    Ok(())
                } else {
                    Err(meta.error("unsupported attribute, expected `path`"))
                }
            })?;
        }
    }

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

    let entity_trait: syn::Path = syn::parse_quote!(#domain_path::Entity);
    let aggregate_trait: syn::Path = syn::parse_quote!(#domain_path::AggregateRoot);
    let domain_event: syn::Path = syn::parse_quote!(#domain_path::DomainEvent);

    let expanded = quote! {
        impl #entity_trait for #name {
            type Id = #id_type;
            fn id(&self) -> Self::Id {
                self.id
            }
        }

        impl #aggregate_trait for #name {
            fn events(&self) -> &[#domain_event] {
                &self.events
            }
            fn clear_events(&mut self) {
                self.events.clear();
            }
            fn add_event(&mut self, event: #domain_event) {
                self.events.push(event);
            }
        }
    };

    Ok(expanded)
}
