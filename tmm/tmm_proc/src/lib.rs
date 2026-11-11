//! A proc macro crate for generating memory maps.

use proc_macro::TokenStream;
use syn::parse_macro_input;

use crate::map::KernelMap;

mod map;

#[proc_macro]
pub fn memory_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as KernelMap);

    match map::impl_kernel_map(input) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
