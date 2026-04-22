use proc_macro2::TokenStream as TokenStream2;
use quote::{quote};
use syn::{
    GenericArgument,
    PathArguments,
    Type,
};

/// 如果 `ty` 是 `Arc<T>`，返回 T；否则返回 ty 本身。
pub fn strip_arc(ty: &Type) -> TokenStream2 {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return quote! { #ty },
    };
    let segs = &path.segments;
    if segs.len() == 1 && segs[0].ident == "Arc" {
        if let PathArguments::AngleBracketed(ab) = &segs[0].arguments {
            if ab.args.len() == 1 {
                if let GenericArgument::Type(inner) = &ab.args[0] {
                    return quote! { #inner };
                }
            }
        }
    }
    quote! { #ty }
}

/// 将驼峰命名法字符串转换为蛇形命名法。
///
/// 在大写字母前插入下划线，并将所有字符转换为小写。
/// 第一个字符前不插入下划线。
///
/// # 参数
///
/// * `s` - 输入的驼峰命名法字符串
///
/// # 返回值
///
/// 转换后的蛇形命名法字符串
///
/// case `DbPool` -> `db_pool`
pub fn camel_to_snake(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        // 在非首字符的大写字母前插入下划线
        if ch.is_uppercase() && i != 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

/// 将驼峰命名法字符串转换为大写蛇形命名法（SCREAMING_SNAKE_CASE）。
///
/// 先转换为蛇形命名法，再将所有字符转为大写。
/// 常用于生成常量名或静态变量名。
///
/// # 参数
///
/// * `s` - 输入的驼峰命名法字符串
///
/// # 返回值
///
/// 转换后的大写蛇形命名法字符串
pub fn camel_to_screaming_snake(s: &str) -> String {
    camel_to_snake(s).to_uppercase()
}


/// 提取 Option<T> 中的 T 类型
pub fn extract_option_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty.clone());
                    }
                }
            }
        }
    }
    None
}

/// 检查类型是否为 Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    extract_option_inner(ty).is_some()
}