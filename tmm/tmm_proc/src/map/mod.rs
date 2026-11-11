use std::collections::HashMap;

use proc_macro::TokenStream;
use syn::{
    braced, parse::Parse, punctuated::Punctuated, spanned::Spanned, token, Expr, Field, FieldValue,
    Ident, LitStr,
};

mod types;

pub use types::*;

pub fn impl_kernel_map(kernel_map: KernelMap) -> syn::Result<TokenStream> {
    Ok(TokenStream::new())
}
