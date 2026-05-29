use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Attribute, Data, DeriveInput, ExprLit, Ident, Lit, Result as SynResult, Token,
};

/// 属性参数：`#[err(code = 1001, msg = "...")]`
struct ErrVariantAttr {
    code: u16,
    msg: String,
}

impl Parse for ErrVariantAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut code = None;
        let mut msg = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            match key.to_string().as_str() {
                "code" => {
                    let lit: ExprLit = input.parse()?;
                    if let Lit::Int(int_lit) = &lit.lit {
                        code = Some(int_lit.base10_parse::<u16>()?);
                    } else {
                        return Err(syn::Error::new_spanned(&lit, "code 必须是整数"));
                    }
                }
                "msg" => {
                    let lit: ExprLit = input.parse()?;
                    if let Lit::Str(s) = &lit.lit {
                        msg = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(&lit, "msg 必须是字符串"));
                    }
                }
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("未知属性 `{}`，仅支持 code, msg", other),
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

/// 属性参数：`#[err(domain = "SYS")]`
struct ErrEnumAttr {
    domain: String,
}

impl Parse for ErrEnumAttr {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let mut domain = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;

            if key == "domain" {
                let lit: ExprLit = input.parse()?;
                if let Lit::Str(s) = &lit.lit {
                    domain = Some(s.value());
                } else {
                    return Err(syn::Error::new_spanned(&lit, "domain 必须是字符串"));
                }
            } else {
                return Err(syn::Error::new_spanned(&key, "仅支持 domain 属性"));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let domain = domain.ok_or_else(|| syn::Error::new(input.span(), "缺少 domain 属性"))?;
        Ok(ErrEnumAttr { domain })
    }
}

/// 从属性列表中找到 `#[err(...)]` 并直接解析为 T
fn find_and_parse_err_attr<T: Parse>(attrs: &[Attribute], _what: &str) -> SynResult<Option<T>> {
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

    // 解析 enum 级别的 #[err(domain = "...")]
    let domain = match find_and_parse_err_attr::<ErrEnumAttr>(&input.attrs, "domain")? {
        Some(attr) => attr.domain,
        None => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "需要在枚举上标注 #[err(domain = \"...\")]",
            ));
        }
    };

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

        // 解析变体级别的 #[err(code = N, msg = "...")]
        let err_attr = match find_and_parse_err_attr::<ErrVariantAttr>(&variant.attrs, "variant")? {
            Some(attr) => attr,
            None => {
                return Err(syn::Error::new_spanned(
                    variant_ident,
                    format!(
                        "变体 `{}` 缺少 #[err(code = N, msg = \"...\")] 属性",
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

    let output = quote! {
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
    };

    Ok(output)
}
