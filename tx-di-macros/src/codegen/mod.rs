//! 代码生成模块
//!
//! 负责将解析后的结构体数据转换为 `impl Component` 和
//! `linkme` 注册条目的 TokenStream。
//!
//! 编排流程：
//! `属性解析 → 字段分类 → 构建 CodeGenContext → 各 codegen 子模块生成片段 → 组装`

pub mod component_impl;
pub mod factory;
pub mod inner_init;
pub mod intercept;
pub mod lifecycle;
pub mod meta_entry;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemStruct, Result as SynResult, Type, Visibility};

use crate::attr::comp_attr::{parse_component_attr_from_attributes, CompAttr};
use crate::classify::fields::{classify_fields, FieldKind};
use crate::type_utils::{
    extract_trait_from_arc, extract_trait_from_option_arc, extract_trait_from_vec_arc, strip_arc_type,
};

/// `#[derive(Component)]` 入口
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    match derive_component_impl(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// 代码生成上下文
///
/// 持有所有已解析的属性数据与已分类的字段集合，
/// 供各 codegen 子模块按需读取。
pub struct CodeGenContext {
    pub struct_name: Ident,
    pub vis: Visibility,
    pub comp_attr: CompAttr,
    /// 全部字段及其分类
    pub fields_info: Vec<(Ident, FieldKind)>,
    /// 普通组件注入字段（字段名, 解包后的内部类型 T）
    pub inject_fields: Vec<(Ident, Type)>,
    /// 可选 trait object 注入字段（字段名, trait 类型）
    pub trait_inject_fields: Vec<(Ident, Type)>,
    /// 必选 trait object 注入字段（字段名, trait 类型）
    pub required_trait_fields: Vec<(Ident, Type)>,
    /// 列表 trait object 注入字段（字段名, trait 类型）
    pub list_trait_fields: Vec<(Ident, Type)>,
}

/// 核心实现：解析 struct，分类字段，生成 `impl Component` 与注册条目
fn derive_component_impl(input: ItemStruct) -> SynResult<TokenStream2> {
    let struct_name = input.ident.clone();
    let vis = input.vis.clone();
    let generics = &input.generics;

    // 解析 #[component(...)] 属性
    let comp_attr = parse_component_attr_from_attributes(&input.attrs)?.unwrap_or_default();

    // 检查是否是泛型结构体
    if !generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "泛型结构体请使用 #[component(for(Type1, Type2))] 指定具体类型参数",
        ));
    }

    // 字段分类
    let fields_info = classify_fields(&input)?;

    // 派生各类字段集合
    let inject_fields: Vec<(Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| match kind {
            FieldKind::Inject { ty } => {
                let inner_ty = strip_arc_type(ty);
                Some((name.clone(), inner_ty))
            }
            _ => None,
        })
        .collect();

    let trait_inject_fields: Vec<(Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| match kind {
            FieldKind::TraitInject { ty } => {
                let trait_ty = extract_trait_from_option_arc(ty)
                    .expect("is_arc_dyn_trait 已验证，提取 trait 类型不应失败");
                Some((name.clone(), trait_ty))
            }
            _ => None,
        })
        .collect();

    let required_trait_fields: Vec<(Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| match kind {
            FieldKind::TraitInjectRequired { ty } => {
                let trait_ty = extract_trait_from_arc(ty)
                    .expect("is_plain_arc_dyn_trait 已验证，提取 trait 类型不应失败");
                Some((name.clone(), trait_ty))
            }
            _ => None,
        })
        .collect();

    let list_trait_fields: Vec<(Ident, Type)> = fields_info
        .iter()
        .filter_map(|(name, kind)| match kind {
            FieldKind::TraitInjectList { ty } => {
                let trait_ty = extract_trait_from_vec_arc(ty)
                    .expect("is_vec_arc_dyn_trait 已验证，提取 trait 类型不应失败");
                Some((name.clone(), trait_ty))
            }
            _ => None,
        })
        .collect();

    let ctx = CodeGenContext {
        struct_name,
        vis,
        comp_attr,
        fields_info,
        inject_fields,
        trait_inject_fields,
        required_trait_fields,
        list_trait_fields,
    };

    // 各子模块生成代码片段
    let component_impl = component_impl::gen_component_impl(&ctx);
    let factory_fn = factory::gen_factory_fn(&ctx);
    let meta_entry = meta_entry::gen_meta_entry(&ctx, factory_fn);

    // 组装最终输出：derive 宏只追加 impl 和 linkme 注册，不重新输出结构体
    Ok(quote! {
        #component_impl
        #meta_entry
    })
}
