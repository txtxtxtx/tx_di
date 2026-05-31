use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Attribute, Data, DeriveInput, Ident, Lit, LitInt, LitStr, Result as SynResult, Token,
};

/// 通用错误码，当只传 `#[err("msg")]` 时作为默认值
const DEFAULT_ERROR_CODE: i32 = -1;

/// 解析 `#[err("...")]` 或 `#[err(N, "...")]` 或 `#[err(code = N, msg = "...")]`
///
/// 三种变体级语法：
/// - `#[err("Config load failed")]`          → code = -1, msg = "Config load failed"
/// - `#[err(0, "Success")]`                  → code = 0,  msg = "Success"
/// - `#[err(code = 0, msg = "Success")]`     → code = 0,  msg = "Success"
struct ErrVariantAttr {
    code: i32,
    msg: String,
}

impl Parse for ErrVariantAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // 情况1: 纯字符串 → `#[err("msg")]`
        if input.peek(LitStr) {
            let s: LitStr = input.parse()?;
            return Ok(ErrVariantAttr { code: DEFAULT_ERROR_CODE, msg: s.value() });
        }

        // 情况2: 整数 → `#[err(N, "msg")]`
        if input.peek(LitInt) {
            let int_lit: LitInt = input.parse()?;
            let code: i32 = int_lit.base10_parse()?;
            let _comma: Token![,] = input.parse()?;
            let s: LitStr = input.parse()?;
            return Ok(ErrVariantAttr { code, msg: s.value() });
        }

        // 情况3: 命名参数 → `#[err(code = N, msg = "...")]`
        let mut code = None;
        let mut msg = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "code" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Int(int_lit) = lit {
                        code = Some(int_lit.base10_parse::<i32>()?);
                    } else {
                        return Err(syn::Error::new_spanned(&lit, "code 必须是整数"));
                    }
                }
                "msg" => {
                    let lit: Lit = input.parse()?;
                    if let Lit::Str(s) = lit {
                        msg = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(&lit, "msg 必须是字符串"));
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("未知属性 `{}`，支持 code, msg", other),
                    ));
                }
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let code = code.ok_or_else(|| syn::Error::new(input.span(), "缺少 code 属性"))?;
        let msg = msg.ok_or_else(|| syn::Error::new(input.span(), "缺少 msg 属性"))?;

        Ok(ErrVariantAttr { code, msg })
    }
}

/// 解析 `#[err("SYS")]` 或 `#[err(domain = "SYS")]`
struct ErrEnumAttr {
    domain: String,
}

impl Parse for ErrEnumAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // `#[err("SYS")]` — 纯字符串
        if input.peek(LitStr) {
            let s: LitStr = input.parse()?;
            return Ok(ErrEnumAttr { domain: s.value() });
        }

        // `#[err(domain = "SYS")]` — 命名参数
        let key: Ident = input.parse()?;
        let _eq: Token![=] = input.parse()?;

        if key != "domain" {
            return Err(syn::Error::new_spanned(&key, "仅支持 domain 属性"));
        }

        let lit: Lit = input.parse()?;
        if let Lit::Str(s) = lit {
            Ok(ErrEnumAttr { domain: s.value() })
        } else {
            Err(syn::Error::new_spanned(&lit, "domain 必须是字符串"))
        }
    }
}

/// 从属性列表中找到 `#[err(...)]` 并直接解析为 T
fn find_and_parse_err_attr<T: Parse>(attrs: &[Attribute]) -> SynResult<Option<T>> {
    for attr in attrs {
        if attr.path().is_ident("err") {
            let parsed: T = attr.parse_args()?;
            return Ok(Some(parsed));
        }
    }
    Ok(None)
}

pub fn derive_code_msg_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_code_msg(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand_code_msg(input: &DeriveInput) -> SynResult<TokenStream2> {
    let enum_name = &input.ident;

    // crate 路径：生成的代码引用 tx_error
    let crate_path = quote! { tx_error };

    // 解析 enum 级别的 #[err("SYS")] 或 #[err(domain = "SYS")]
    let domain = match find_and_parse_err_attr::<ErrEnumAttr>(&input.attrs)? {
        Some(attr) => attr.domain,
        None => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                r#"需要在枚举上标注 #[err("SYS")] 或 #[err(domain = "SYS")]"#,
            ));
        }
    };

    // 解析可选的 #[ie(tx_error::IE)] 属性
    let ie_path: Option<syn::Path> = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("ie") {
            attr.parse_args::<syn::Path>().ok()
        } else {
            None
        }
    });

    // 解析每个变体
    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "CodeMsg 仅支持枚举类型",
            ));
        }
    };

    let mut match_arms = Vec::new();

    for variant in variants {
        let variant_ident = &variant.ident;

        let err_attr = match find_and_parse_err_attr::<ErrVariantAttr>(&variant.attrs)? {
            Some(attr) => attr,
            None => {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    format!(
                        r#"变体 `{}` 缺少 #[err(N, "msg")] 或 #[err("msg")] 属性"#,
                        variant_ident
                    ),
                ));
            }
        };

        let code = err_attr.code;
        let msg = &err_attr.msg;

        match_arms.push(quote! {
            Self::#variant_ident => #crate_path::AppErrCode::new(#domain, #code, #msg)
        });
    }

    let display_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let ident = &v.ident;
            quote! {
                Self::#ident => {
                    let code = <Self as #crate_path::CodeMsg>::err_code(*self);
                    write!(f, "{code}")
                }
            }
        })
        .collect();

    let mut output = quote! {
        impl #crate_path::CodeMsg for #enum_name {
            fn err_code(self) -> #crate_path::AppErrCode {
                match self {
                    #( #match_arms ),*
                }
            }
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #( #display_arms ),*
                }
            }
        }

        impl From<#enum_name> for #crate_path::AppError {
            fn from(e: #enum_name) -> Self {
                #crate_path::AppError::from_code(e)
            }
        }

        // Display + Debug 已由本宏 + derive 生成，Error 可直接实现
        impl std::error::Error for #enum_name {}
    };

    // 如果指定了 #[ie(...)]，额外生成 From<EnumName> for IE
    if let Some(ie_ty) = ie_path {
        output.extend(quote! {
            impl From<#enum_name> for #ie_ty {
                fn from(e: #enum_name) -> Self {
                    #ie_ty::Business(<#enum_name as #crate_path::CodeMsg>::err_code(e))
                }
            }
        });
    }

    Ok(output)
}
