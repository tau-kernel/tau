use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use syn::{
    braced, parse::Parse, punctuated::Punctuated, spanned::Spanned, token, Expr, ExprStruct, Field,
    FieldValue, Ident, ItemFn, ItemStatic, ItemStruct, LitStr, Token, Visibility,
};

mod codegen;
mod parse;

pub use parse::*;

pub fn impl_kernel_map(kernel_map: KernelMap) -> syn::Result<TokenStream> {
    Ok(codegen::impl_map(kernel_map))
}
