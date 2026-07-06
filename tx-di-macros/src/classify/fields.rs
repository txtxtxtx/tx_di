//! 字段分类逻辑
//!
//! 遍历结构体字段，根据类型形态和 `#[tx_cst]` 属性将其归类为
//! `FieldKind`，供后续代码生成使用。

use syn::{Expr, Ident, ItemStruct, Result as SynResult, Type};

use crate::attr::field_attr::{extract_inject_expr, has_skip_attr};
use crate::type_utils::{is_arc_dyn_trait, is_option_type, is_plain_arc_dyn_trait, is_vec_arc_dyn_trait};

/// 字段注入类别
///
/// 由字段类型形态与 `#[tx_cst]` 属性共同决定。
#[derive(Clone)]
pub enum FieldKind {
    /// 普通组件注入（`Arc<T>`），从 `Deps` 元组解构
    Inject { ty: Type },
    /// 可选 trait object 注入（`Option<Arc<dyn Trait>>`）
    TraitInject { ty: Type },
    /// 必选 trait object 注入（`Arc<dyn Trait>`）
    TraitInjectRequired { ty: Type },
    /// 列表 trait object 注入（`Vec<Arc<dyn Trait>>`），注入所有实现
    TraitInjectList { ty: Type },
    /// 自定义表达式赋值（`#[tx_cst(expr)]`）
    Custom { expr: Expr },
    /// 可选普通依赖（`Option<T>`，非 trait），build 时填 None
    Optional { _ty: Type },
    /// 跳过注入（`#[tx_cst(skip)]`），使用 Default
    Skip,
}

/// 对结构体所有字段进行分类
///
/// 返回 `Vec<(字段名, FieldKind)>`。仅支持具名字段。
pub fn classify_fields(input: &ItemStruct) -> SynResult<Vec<(Ident, FieldKind)>> {
    let mut fields_info: Vec<(Ident, FieldKind)> = Vec::new();

    for field in &input.fields {
        let ident = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(&field.ty, "#[tx_cst] 只支持具名字段"))?;
        let inject_expr = extract_inject_expr(&field.attrs)?;
        let kind = if has_skip_attr(&field.attrs) {
            FieldKind::Skip
        } else if is_plain_arc_dyn_trait(&field.ty) {
            // Arc<dyn Trait> — 必选 trait object 注入
            FieldKind::TraitInjectRequired { ty: field.ty.clone() }
        } else if is_vec_arc_dyn_trait(&field.ty) {
            // Vec<Arc<dyn Trait>> — 列表 trait object 注入
            FieldKind::TraitInjectList { ty: field.ty.clone() }
        } else if is_arc_dyn_trait(&field.ty) {
            // Option<Arc<dyn Trait>> — 可选 trait object 注入
            FieldKind::TraitInject { ty: field.ty.clone() }
        } else if is_option_type(&field.ty) {
            FieldKind::Optional { _ty: field.ty.clone() }
        } else if let Some(expr) = inject_expr {
            FieldKind::Custom { expr }
        } else {
            FieldKind::Inject { ty: field.ty.clone() }
        };
        fields_info.push((ident.clone(), kind));
    }

    Ok(fields_info)
}
