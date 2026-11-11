use core::fmt::Debug;
use std::collections::HashMap;

use proc_macro2::Span;
use syn::{
    parse::{Parse, Peek},
    Ident, LitInt, LitStr, Path, Token,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryAmount {
    value: u64,
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

impl Parse for MemoryAmount {
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
        Ok(MemoryAmount { value })
    }
}

pub type RegionFields = syn::punctuated::Punctuated<RegionField, Token![,]>;

pub struct RegionField {
    name: Ident,
    colon: Token![:],
    value: Value,
}

impl Parse for RegionField {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let colon: Token![:] = input.parse()?;
        let value: Value = input.parse()?;
        Ok(RegionField { name, colon, value })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Memory {
    Literal(MemoryAmount),
    Ident(Ident),
}

#[derive(Clone)]
pub enum Value {
    Size(Memory),
    Str(LitStr),
}

impl Parse for Value {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitInt) {
            let size: MemoryAmount = input.parse()?;
            Ok(Value::Size(Memory::Literal(size)))
        } else if input.peek(syn::LitStr) {
            let lit_str: LitStr = input.parse()?;
            Ok(Value::Str(lit_str))
        } else if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            Ok(Value::Size(Memory::Ident(ident)))
        } else {
            Err(syn::Error::new(
                input.span(),
                "Expected a size, string literal, or identifier",
            ))
        }
    }
}

struct FieldReader {
    fields: HashMap<String, (Value, Span)>,
}

impl Parse for FieldReader {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let fields = RegionFields::parse_terminated(input)?;
        let mut field_map = HashMap::new();
        for field in fields {
            let name = field.name.to_string();
            if let Some((_, old_span)) =
                field_map.insert(name.clone(), (field.value, field.name.span()))
            {
                let mut err = syn::Error::new(old_span, format!("Duplicate field `{}`", name));
                err.combine(syn::Error::new(field.name.span(), "redefined here"));
                return Err(err);
            }
        }

        Ok(FieldReader { fields: field_map })
    }
}

impl FieldReader {
    pub fn get_memory(&self, name: &str) -> Result<Memory, syn::Error> {
        match self.fields.get(name) {
            Some((v, span)) => match v {
                Value::Size(mem) => Ok(mem.clone()),
                _ => Err(syn::Error::new(
                    *span,
                    format!("Field `{}` is not a memory size", name),
                )),
            },
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("Field `{}` not found", name),
            )),
        }
    }

    pub fn get_string(&self, name: &str) -> Result<LitStr, syn::Error> {
        match self.fields.get(name) {
            Some((v, span)) => match v {
                Value::Str(lit_str) => Ok(lit_str.clone()),
                _ => Err(syn::Error::new(
                    *span,
                    format!("Field `{}` is not a string literal", name),
                )),
            },
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("Field `{}` not found", name),
            )),
        }
    }
}

pub struct RegionRequest {
    ident: Ident,
    pretty_name: LitStr,
    size: Memory,
    align: Memory,
}

impl Debug for RegionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionRequest")
            .field("ident", &self.ident)
            .field("pretty_name", &self.pretty_name.value())
            .field("size", &self.size)
            .field("align", &self.align)
            .finish()
    }
}

impl Parse for RegionRequest {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let contents;
        syn::braced!(contents in input);
        let fields: FieldReader = contents.parse()?;

        Ok(Self {
            ident,
            pretty_name: fields.get_string("name")?,
            size: fields.get_memory("size")?,
            align: fields.get_memory("align")?,
        })
    }
}

#[derive(Debug)]
struct ReservedKeyword;

impl ReservedKeyword {
    fn peek(input: syn::parse::ParseStream) -> bool {
        input.peek(Ident) && input.peek2(Ident)
    }
}

impl Parse for ReservedKeyword {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let kw = input.parse::<Ident>()?;
        println!("{:?}", input);
        if kw == "reserved" {
            if !input.peek(Ident) {
                return Err(syn::Error::new(
                    input.span(),
                    "Expected an identifier after `reserved`",
                ));
            }
            Ok(ReservedKeyword)
        } else {
            Err(syn::Error::new(
                kw.span(),
                "Expected the keyword `reserved`",
            ))
        }
    }
}

pub struct ReservedRegion {
    ident: Ident,
    pretty_name: LitStr,
    start: Memory,
    end: Memory,
}
impl Debug for ReservedRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionRequest")
            .field("ident", &self.ident)
            .field("pretty_name", &self.pretty_name.value())
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}
impl Parse for ReservedRegion {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<ReservedKeyword>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let contents;
        syn::braced!(contents in input);
        let fields: FieldReader = contents.parse()?;

        Ok(Self {
            ident: name,
            pretty_name: fields.get_string("name")?,
            start: fields.get_memory("start")?,
            end: fields.get_memory("end")?,
        })
    }
}
#[derive(Debug)]
pub enum Region {
    Request(RegionRequest),
    Reserved(ReservedRegion),
}

impl Parse for Region {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if ReservedKeyword::peek(input) {
            let reserved_region: ReservedRegion = input.parse()?;
            Ok(Region::Reserved(reserved_region))
        } else {
            let region_request: RegionRequest = input.parse()?;
            Ok(Region::Request(region_request))
        }
    }
}

pub type Regions = syn::punctuated::Punctuated<Region, Token![,]>;

#[derive(Debug)]
pub struct KernelMap {
    requested_regions: Vec<RegionRequest>,
    reserved_regions: Vec<ReservedRegion>,
}

impl Parse for KernelMap {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let regions = Regions::parse_terminated(input)?;
        let mut requested = vec![];
        let mut reserved = vec![];
        for region in regions {
            match region {
                Region::Request(r) => requested.push(r),
                Region::Reserved(r) => reserved.push(r),
            }
        }

        Ok(KernelMap {
            requested_regions: requested,
            reserved_regions: reserved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse::Parser, parse_str};

    #[test]
    fn test_parse_memory_amount() {
        let input = "16 MiB";
        let memory_amount: MemoryAmount = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 16 * 1024 * 1024);

        let input = "1 GiB";
        let memory_amount: MemoryAmount = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 1024 * 1024 * 1024);

        let input = "512 KiB";
        let memory_amount: MemoryAmount = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 512 * 1024);

        let input = "4096 B";
        let memory_amount: MemoryAmount = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 4096);
    }

    #[test]
    fn test_parse_memory_amount_invalid_unit() {
        let input = "16 MB"; // Invalid unit
        let result: Result<MemoryAmount, syn::Error> = parse_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_reserved_keyword() {
        // Reserve is a little weird since it's somewhat context dependant-
        // reserved is a valid region name, so it MUST check that there's an ident after.
        let input = "reserved KERNEL";
        struct Test;
        impl Parse for Test {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                input.parse::<ReservedKeyword>()?;
                input.parse::<Ident>()?;
                Ok(Test)
            }
        }
        parse_str::<Test>(input).unwrap();
        let bad = "reserved";
        parse_str::<ReservedKeyword>(bad).unwrap_err();
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
        println!("{:?}", parse);
    }
}
