use core::fmt::Debug;
use std::collections::HashMap;

use proc_macro2::Span;
use syn::{
    parse::{Parse, Peek},
    punctuated::Punctuated,
    Attribute, Ident, LitInt, LitStr, Meta, Path, Token,
};

mod reader;
mod region;
mod value;
pub use region::*;
pub use value::*;

mod kw {
    syn::custom_keyword!(reserved);
}

type RegionInfoList = Punctuated<RegionInfo, Token![,]>;

#[derive(Debug)]
pub struct KernelMap {
    pub regions: Vec<RegionInfo>,
}

impl Parse for KernelMap {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let regions = RegionInfoList::parse_terminated(input)?;
        Ok(Self {
            regions: regions.iter().cloned().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::map::codegen;

    use super::*;
    use syn::{parse::Parser, parse_str};

    #[test]
    fn test_parse_memory_amount_invalid_unit() {
        let input = "16 MB"; // Invalid unit
        let result: Result<MemLit, syn::Error> = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_yay_cool() {
        let contents = "SHM = {
            name: \"SHared Memory\",
            size: 1 L4,
            align: 1 L4,
        },
        LOCAL_MANAGED = {
            name: \"Local Managed\",
            size: 8 GiB,
            align: 2 GiB,
        },
        LOCAL_HEAP = {
            name: \"Local Heap\",
            size: 4 GiB,
            align: 1 GiB,
        },
        SHM_HEAP = {
            name: \"SHared Memory Heap\",
            size: 4 GiB,
            align: 1 GiB,
        },
        reserved KERNEL = {
            name: \"Kernel\",
            start: _KERNEL_START,
            end: _KERNEL_END,
        }
    ";

        let parse = parse_str::<KernelMap>(contents).unwrap();
        println!("{:#?}", parse);

        let mimpl = codegen::impl_map(parse);
        println!("{}", mimpl);
    }

    #[test]
    fn test_parse_meta() {
        let test = "/// test 1.
        TEST_1 = { name: \"test1\",
                    size: 1 L4,
                    align: 1 L4,
    }";
        let parse = parse_str::<KernelMap>(test).unwrap();
        println!("{:#?}", parse);
    }
}
