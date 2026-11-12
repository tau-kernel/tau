use core::fmt::Debug;
use std::collections::HashMap;

use proc_macro2::Span;
use syn::{
    parse::{Parse, Peek},
    punctuated::Punctuated,
    Attribute, Ident, LitInt, LitStr, Meta, Path, Token,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MemLit {
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
        Ok(MemLit { value })
    }
}

pub type RegionFields = syn::punctuated::Punctuated<RegionField, Token![,]>;

pub struct RegionField {
    name: Ident,
    value: Value,
}

impl Parse for RegionField {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: Value = input.parse()?;
        Ok(RegionField { name, value })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Memory {
    Literal(MemLit),
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
            let size: MemLit = input.parse()?;
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
    pub fn get_memory(&mut self, name: &str) -> Result<Memory, syn::Error> {
        match self.fields.remove(name) {
            Some((v, span)) => match v {
                Value::Size(mem) => Ok(mem.clone()),
                _ => Err(syn::Error::new(
                    span,
                    format!("Field `{}` is not a memory size", name),
                )),
            },
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("Field `{}` not found", name),
            )),
        }
    }

    pub fn get_string(&mut self, name: &str) -> Result<LitStr, syn::Error> {
        match self.fields.remove(name) {
            Some((v, span)) => match v {
                Value::Str(lit_str) => Ok(lit_str.clone()),
                _ => Err(syn::Error::new(
                    span,
                    format!("Field `{}` is not a string literal", name),
                )),
            },
            None => Err(syn::Error::new(
                Span::call_site(),
                format!("Field `{}` not found", name),
            )),
        }
    }

    pub fn check_empty(&self) -> Result<(), syn::Error> {
        let mut err: Result<(), syn::Error> = Ok(());
        for (_, (_, span)) in &self.fields {
            if let Err(err) = &mut err {
                err.combine(syn::Error::new(*span, "Undefined field"));
            } else {
                err = Err(syn::Error::new(*span, "Undefined field"))
            }
        }
        err
    }
}

#[derive(Clone)]
pub struct RegionInfo {
    attribs: Vec<Attribute>,
    ident: Ident,
    pretty_name: LitStr,
    region: Region,
}

impl Debug for RegionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionInfo")
            .field("attribs", &self.attribs.len())
            .field("ident", &self.ident)
            .field("pretty_name", &self.pretty_name.value())
            .field("region", &self.region)
            .finish()
    }
}

impl Parse for RegionInfo {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attribs = input.call(Attribute::parse_outer)?;
        let is_reserved = input.peek(kw::reserved);

        if is_reserved {
            input.parse::<kw::reserved>()?;
        }

        let name = input.parse::<Ident>()?;
        input.parse::<Token![=]>()?;
        let region;
        syn::braced!(region in input);
        let mut reader = region.parse::<FieldReader>()?;
        let pretty_name = reader.get_string("name")?;
        let region = Region::from_reader(reader, is_reserved)?;

        Ok(Self {
            attribs,
            ident: name,
            pretty_name,
            region,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Region {
    Request(RequestedRegion),
    Reserved(ReservedRegion),
}

impl Region {
    fn from_reader(mut reader: FieldReader, is_reserved: bool) -> Result<Self, syn::Error> {
        if is_reserved {
            let start = reader.get_memory("start")?;
            let end = reader.get_memory("end")?;
            reader.check_empty()?;
            return Ok(Self::Reserved(ReservedRegion { start, end }));
        }
        let size = reader.get_memory("size")?;
        let align = reader.get_memory("align")?;
        reader.check_empty()?;
        Ok(Self::Request(RequestedRegion { size, align }))
    }
}

#[derive(Debug, Clone)]
pub struct RequestedRegion {
    size: Memory,
    align: Memory,
}

mod kw {
    syn::custom_keyword!(reserved);
}

#[derive(Debug, Clone)]
pub struct ReservedRegion {
    start: Memory,
    end: Memory,
}

type RegionInfoList = Punctuated<RegionInfo, Token![,]>;

#[derive(Debug)]
pub struct KernelMap {
    regions: Vec<RegionInfo>,
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
    use super::*;
    use syn::{parse::Parser, parse_str};

    #[test]
    fn test_parse_memory_amount() {
        let input = "16 MiB";
        let memory_amount: MemLit = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 16 * 1024 * 1024);

        let input = "1 GiB";
        let memory_amount: MemLit = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 1024 * 1024 * 1024);

        let input = "512 KiB";
        let memory_amount: MemLit = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 512 * 1024);

        let input = "4096 B";
        let memory_amount: MemLit = parse_str(input).unwrap();
        assert_eq!(memory_amount.value, 4096);
    }

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
        println!("{:?}", parse);
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
