use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse::Parse, spanned::Spanned, Ident, LitInt, LitStr, Path};

#[derive(Clone, Debug)]
pub struct MemLit {
    pub lit: LitInt,
}

fn unit_multiplier(unit: &str) -> Option<u64> {
    match unit {
        "B" => Some(1),                           // 1 byte
        "KiB" => Some(1024),                      // 1 KiB
        "MiB" => Some(1024 * 1024),               // 1 MiB
        "GiB" | "L3" => Some(1024 * 1024 * 1024), // 1 GiB
        "L1" => Some(4096),                       // 4 KiB
        "L2" => Some(512 * 4096),                 // 2 MiB
        "L4" => Some(512 * 512 * 512 * 4096),     // 512 GiB
        _ => None,
    }
}

impl Parse for MemLit {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lit: syn::LitInt = input.parse()?;
        let multi = match input.parse::<Option<Ident>>()? {
            Some(multi) => unit_multiplier(&multi.to_string())
                .ok_or_else(|| syn::Error::new(multi.span(), "Invalid unit for memory size"))?,
            None => 1,
        };
        let value = lit.base10_parse::<u64>()?;
        let value = value.checked_mul(multi).ok_or_else(|| {
            syn::Error::new(
                lit.span(),
                "Memory size overflowed when applying unit multiplier",
            )
        })?;
        Ok(MemLit {
            lit: LitInt::new(&format!("{}", value), lit.span()),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Memory {
    Literal(MemLit),
    External(Path),
}

impl ToTokens for Memory {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Memory::External(e) => e.to_tokens(tokens),
            Memory::Literal(lit) => lit.lit.to_tokens(tokens),
        }
    }
}

#[derive(Clone)]
pub enum Value {
    Size(Memory),
    Str(LitStr),
}

impl Parse for Value {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitInt) {
            let size: MemLit = input.parse()?;
            return Ok(Value::Size(Memory::Literal(size)));
        } else if input.peek(syn::LitStr) {
            let lit_str: LitStr = input.parse()?;
            return Ok(Value::Str(lit_str));
        }
        let path = input.parse()?;
        Ok(Value::Size(Memory::External(path)))
    }
}
