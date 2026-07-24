//! 类型检测与提取工具
//!
//! 提供 `Arc<T>`、`Option<T>`、`Arc<dyn Trait>`、`Option<Arc<dyn Trait>>` 等
//! 类型的检测与内部类型提取，供字段分类使用。

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// 如果 `ty` 是 `Arc<T>`，返回 T 的 Type；否则返回 ty 本身。
pub fn strip_arc_type(ty: &Type) -> Type {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return ty.clone(),
    };
    let segs = &path.segments;
    if segs.len() == 1
        && segs[0].ident == "Arc"
        && let PathArguments::AngleBracketed(ab) = &segs[0].arguments
        && ab.args.len() == 1
        && let GenericArgument::Type(inner) = &ab.args[0]
    {
        return inner.clone();
    }
    ty.clone()
}

/// 提取 `Option<T>` 中的 T 类型
pub fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return Some(inner_ty.clone());
    }
    None
}

/// 检查类型是否为 `Option<T>`
pub fn is_option_type(ty: &Type) -> bool {
    extract_option_inner(ty).is_some()
}

/// 从 `Arc<dyn Trait>` 中提取 `dyn Trait` 的 Type
///
/// 用于 required trait inject 字段的内联注入。
pub fn extract_trait_from_arc(ty: &Type) -> Option<Type> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return None,
    };
    let segs = &path.segments;
    if segs.len() != 1 || segs[0].ident != "Arc" {
        return None;
    }
    if let PathArguments::AngleBracketed(ab) = &segs[0].arguments
        && let Some(GenericArgument::Type(trait_ty @ Type::TraitObject(_))) = ab.args.first()
    {
        return Some(trait_ty.clone());
    }
    None
}

/// 从 `Option<Arc<dyn Trait>>` 中提取 `dyn Trait` 的 Type
///
/// 用于 trait inject 字段的 inner_init 生成。
pub fn extract_trait_from_option_arc(ty: &Type) -> Option<Type> {
    // 先提取 Option<T> 的 T
    let inner = extract_option_inner(ty)?;
    // T 应该是 Arc<dyn Trait>
    let path = match &inner {
        Type::Path(tp) => &tp.path,
        _ => return None,
    };
    let segs = &path.segments;
    if segs.len() != 1 || segs[0].ident != "Arc" {
        return None;
    }
    if let PathArguments::AngleBracketed(ab) = &segs[0].arguments
        && let Some(GenericArgument::Type(trait_ty @ Type::TraitObject(_))) = ab.args.first()
    {
        return Some(trait_ty.clone());
    }
    None
}

/// 检查类型是否为 `Option<Arc<dyn Trait>>` 形式
pub fn is_arc_dyn_trait(ty: &Type) -> bool {
    extract_trait_from_option_arc(ty).is_some()
}

/// 检查类型是否为 `Arc<dyn Trait>` 形式（无 Option 包裹）
pub fn is_plain_arc_dyn_trait(ty: &Type) -> bool {
    extract_trait_from_arc(ty).is_some()
}

/// 从 `Vec<Arc<dyn Trait>>` 中提取 `dyn Trait` 的 Type
///
/// 用于列表 trait inject 字段的 inner_init 生成。
pub fn extract_trait_from_vec_arc(ty: &Type) -> Option<Type> {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return None,
    };
    let segs = &path.segments;
    if segs.len() != 1 || segs[0].ident != "Vec" {
        return None;
    }
    if let PathArguments::AngleBracketed(ab) = &segs[0].arguments
        && let Some(GenericArgument::Type(arc_ty)) = ab.args.first()
    {
        // arc_ty 应该是 Arc<dyn Trait>
        return extract_trait_from_arc(arc_ty);
    }
    None
}

/// 检查类型是否为 `Vec<Arc<dyn Trait>>` 形式
pub fn is_vec_arc_dyn_trait(ty: &Type) -> bool {
    extract_trait_from_vec_arc(ty).is_some()
}

/// 将类型按 `Arc<T>` 解包后生成 TokenStream（若不是 Arc 则原样输出）
///
/// 保留用于需要在宏展开中输出内部类型的场景。
#[allow(dead_code)]
pub fn strip_arc_tokens(ty: &Type) -> TokenStream2 {
    let inner = strip_arc_type(ty);
    quote! { #inner }
}
