//! 类型检测与提取工具
//!
//! 提供 `Arc<T>`、`Option<T>`、`Arc<dyn Trait>`、`Option<Arc<dyn Trait>>` 等
//! 类型的检测与内部类型提取，供字段分类使用。

use syn::{GenericArgument, PathArguments, Type};

/// 如果 `ty` 是 `Arc<T>`（支持 `Arc<T>` 或 `std::sync::Arc<T>` 等路径），
/// 返回 T 的 Type；否则返回 ty 本身。
pub fn strip_arc_type(ty: &Type) -> Type {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return ty.clone(),
    };
    let segs = &path.segments;
    if let Some(last) = segs.last()
        && last.ident == "Arc"
        && let PathArguments::AngleBracketed(ab) = &last.arguments
        && ab.args.len() == 1
        && let GenericArgument::Type(inner) = &ab.args[0]
    {
        return inner.clone();
    }
    ty.clone()
}

/// 提取 `Option<T>` 中的 T 类型（支持任意路径前缀如 `std::option::Option<T>`）
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
    let last = segs.last()?;
    if last.ident != "Arc" {
        return None;
    }
    if let PathArguments::AngleBracketed(ab) = &last.arguments
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
    extract_trait_from_arc(&inner)
}

/// 检查类型是否为 `Option<Arc<dyn Trait>>` 形式
pub fn is_option_arc_dyn_trait(ty: &Type) -> bool {
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
    let last = segs.last()?;
    if last.ident != "Vec" {
        return None;
    }
    if let PathArguments::AngleBracketed(ab) = &last.arguments
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

/// 检查类型是否可参与 DI 注入（`Arc<T>`、`Option<Arc<T>>` 等）
///
/// 用于字段分类的兜底检查：非 Arc 形 + 无 #[tx_cst] 的字段应给出编译错误。
pub fn is_arc_like(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            // Arc<...> / Option<...> / Vec<...>
            return last.ident == "Arc" || last.ident == "Option" || last.ident == "Vec";
        }
    }
    false
}

/// 检查类型是否为 `Option<Arc<T>>`（非 trait object）形式
pub fn is_option_arc_type(ty: &Type) -> bool {
    let inner = extract_option_inner(ty);
    inner
        .as_ref()
        .map(|i| !extract_trait_from_arc(i).is_some()) // 非 trait object
        .unwrap_or(false)
        && inner
            .map(|i| {
                if let Type::Path(tp) = &i {
                    tp.path.segments.last().map(|s| s.ident == "Arc").unwrap_or(false)
                } else {
                    false
                }
            })
            .unwrap_or(false)
}
