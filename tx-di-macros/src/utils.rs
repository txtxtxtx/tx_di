//! 表示 Rust 结构体定义（包括命名结构体、元组结构体和单元结构体）的 AST 节点。
//!
//! 该结构体对应 `syn::ItemStruct`，用于解析和操作 `struct` 定义。
//! 它包含了结构体的所有元数据，是 Rust 语法树中结构体定义的核心类型。
//! ```rust,ignore
//! pub struct ItemStruct {
//!     /// 结构体上的属性列表，例如 `#[derive(Debug)]`、`#[cfg(...)]`、文档注释等。
//!     /// 属性可以影响结构体的编译行为、派生实现和文档生成。
//!     pub attrs: Vec<Attribute>,
//!
//!     /// 结构体的可见性修饰符，例如 `pub`、`pub(crate)` 或无修饰符（默认私有）。
//!     /// 控制结构体在当前 crate 内外的访问权限。
//!     pub vis: Visibility,
//!
//!     /// `struct` 关键字本身的 Token，用于保留源代码中的位置信息（Span），
//!     /// 便于编译器进行错误报告和代码映射。
//!     pub struct_token: Token![struct],
//!
//!     /// 结构体的名称标识符（Identifier），如 `MyStruct`。
//!     /// 该名称在作用域内必须唯一，用于引用该结构体类型。
//!     pub ident: Ident,
//!
//!     /// 结构体的泛型参数声明，包括生命周期参数、类型参数和相应的约束（bounds）。
//!     /// 例如 `struct Foo<T: Clone, 'a>` 中的 `<T: Clone, 'a>` 部分。
//!     pub generics: Generics,
//!
//!     /// 结构体的字段集合，定义结构体包含的数据。
//!     /// - `Fields::Named`：命名结构体，如 `struct Foo { bar: u8 }`
//!     /// - `Fields::Unnamed`：元组结构体，如 `struct Foo(u8, u32)`
//!     /// - `Fields::Unit`：单元结构体，无任何字段，如 `struct Foo;`
//!     pub fields: Fields,
//!
//!     /// 分号 Token，仅当结构体是单元结构体（`struct Foo;`）时存在。
//!     /// 对于命名结构体或元组结构体（带有大括号或括号），此字段为 `None`。
//!     /// 该 Token 用于区分单元结构体与其它形式，并保留其在源代码中的位置。
//!     pub semi_token: Option<Token![;]>,
//! }
//! ```
//! 表示 Rust 属性（Attribute）的 AST 节点，例如 `#[derive(Debug)]` 或 `#[cfg(feature = "foo")]`。
//!
//! 属性是附加在项（如结构体、函数、模块等）上的元数据，用于指导编译器行为、
//! 条件编译、派生实现或文档生成。
//! ```rust,ignore
//! pub struct Attribute {
//!     /// `#` 符号的 Token，表示属性开始，例如 `#[...]` 中的 `#`。
//!     /// 保留位置信息用于错误报告和代码映射。
//!     pub pound_token: Token![#],
//!
//!     /// 属性的样式（风格），决定属性的语法形式。
//!     /// - `AttrStyle::Outer`：外层属性，如 `#[...]`，通常应用于整个项。
//!     /// - `AttrStyle::Inner`：内层属性，如 `#![...]`，通常应用于所在的模块或 crate。
//!     pub style: AttrStyle,
//!
//!     /// 方括号 `[...]` 的 Token，用于界定属性的内部内容。
//!     /// 例如 `#[derive(Debug)]` 中的方括号对。
//!     pub bracket_token: token::Bracket,
//!
//!     /// 属性的元数据（Meta），包含属性的路径和参数。
//!     /// 例如 `derive(Debug)` 中的 `derive` 是路径，`Debug` 是参数。
//!     pub meta: Meta,
//! }
//!
//! /// 表示 Rust 项的可见性（Visibility），决定该项在模块或 crate 外部的可访问性。
//! ///
//! /// 可见性可以是公有的（`pub`）、受限的（如 `pub(crate)`）或继承的（默认私有）。
//! pub enum Visibility {
//!     /// 完全公有的可见性：`pub`，表示该项在 crate 外部也可访问。
//!     Public(Token![pub]),
//!
//!     /// 受限的可见性，将访问限制在特定路径范围内。
//!     /// 形式包括 `pub(self)`、`pub(super)`、`pub(crate)` 或 `pub(in some::module)`。
//!     Restricted(VisRestricted),
//!
//!     /// 继承的可见性，通常表示私有（除非其所在模块允许）。
//!     /// 这是默认可见性，即不显式指定 `pub` 时的行为。
//!     Inherited,
//! }
//!
//! /// 表示 Rust 的标识符（Identifier），如变量名、类型名、函数名等。
//! ///
//! /// 该结构封装了编译器内部的标识符表示，并提供比较、哈希和调试支持。
//! #[derive(Clone)]
//! pub struct Ident {
//!     /// 内部的实际标识符表示（依赖于 `proc_macro` 的具体实现）。
//!     /// 该字段存储了标识符的字符串内容和 Span 信息。
//!     inner: imp::Ident,
//!
//!     /// 为了确保 `ProcMacroAutoTraits` 的自动 trait 实现（如 Send/Sync）正确工作而保留的标记。
//!     _marker: ProcMacroAutoTraits,
//! }
//!
//! /// 表示结构体或枚举的泛型参数声明，包括生命周期、类型参数及其约束。
//! ///
//! /// 例如 `struct Foo<'a, T: Clone, U = u8>` 中的 `<'a, T: Clone, U = u8>` 部分。
//! pub struct Generics {
//!     /// 左尖括号 `<` 的 Token，若存在则表明有泛型参数。
//!     /// 当没有泛型参数时，此字段为 `None`。
//!     pub lt_token: Option<Token![<]>,
//!
//!     /// 泛型参数列表，每个参数可以是生命周期参数（`Lifetime`）、类型参数（`TypeParam`）或常量参数（`ConstParam`）。
//!     /// 列表中的元素用逗号 `,` 分隔（由 `Punctuated` 维护分隔符）。
//!     pub params: Punctuated<GenericParam, Token![,]>,
//!
//!     /// 右尖括号 `>` 的 Token，与 `lt_token` 配对。
//!     /// 若 `lt_token` 为 `None`，则此字段也为 `None`。
//!     pub gt_token: Option<Token![>]>,
//!
//!     /// `where` 子句，用于对泛型参数添加额外的约束，例如 `where T: Display, U: Clone`。
//!     /// 若没有 `where` 子句，此字段为 `None`。
//!     pub where_clause: Option<WhereClause>,
//! }
//!
//! /// 表示结构体或枚举变体的字段集合，区分命名字段、未命名字段和单元字段。
//! ///
//! /// 对应于结构体定义的三种形式：
//! /// - 命名结构体（`struct S { x: u8, y: u8 }`）
//! /// - 元组结构体（`struct S(u8, u8)`）
//! /// - 单元结构体（`struct S;`）
//! pub enum Fields {
//!     /// 命名字段，适用于普通结构体或结构体变体，如 `Point { x: f64, y: f64 }`。
//!     /// 每个字段都有名称和类型。
//!     Named(FieldsNamed),
//!
//!     /// 未命名字段，适用于元组结构体或元组变体，如 `Some(T)`。
//!     /// 字段按顺序排列，只有类型没有名称。
//!     Unnamed(FieldsUnnamed),
//!
//!     /// 单元结构体或单元变体，不包含任何字段，如 `None`。
//!     Unit,
//! }

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote};
use syn::{
    GenericArgument,
    PathArguments,
    Type,
};

/// 如果 `ty` 是 `Arc<T>`，返回 T 的 TokenStream；否则返回 ty 本身。
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

/// 如果 `ty` 是 `Arc<T>`，返回 T 的 Type；否则返回 ty 本身。
pub fn strip_arc_type(ty: &Type) -> Type {
    let path = match ty {
        Type::Path(tp) => &tp.path,
        _ => return ty.clone(),
    };
    let segs = &path.segments;
    if segs.len() == 1 && segs[0].ident == "Arc" {
        if let PathArguments::AngleBracketed(ab) = &segs[0].arguments {
            if ab.args.len() == 1 {
                if let GenericArgument::Type(inner) = &ab.args[0] {
                    return inner.clone();
                }
            }
        }
    }
    ty.clone()
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
    if let PathArguments::AngleBracketed(ab) = &segs[0].arguments {
        if let Some(GenericArgument::Type(trait_ty @ Type::TraitObject(_))) = ab.args.first() {
            return Some(trait_ty.clone());
        }
    }
    None
}

/// 检查类型是否为 `Option<Arc<dyn Trait>>` 形式
pub fn is_arc_dyn_trait(ty: &Type) -> bool {
    extract_trait_from_option_arc(ty).is_some()
}