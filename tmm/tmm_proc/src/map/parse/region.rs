use syn::{parse::Parse, Attribute, Ident, LitStr, Token};

use crate::map::{
    parse::{kw, reader::FieldReader},
    Memory,
};

#[derive(Clone, Debug)]
pub struct RegionInfo {
    pub attribs: Vec<Attribute>,
    pub ident: Ident,
    pub pretty_name: LitStr,
    pub region: Region,
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
    pub size: Memory,
    pub align: Memory,
}

#[derive(Debug, Clone)]
pub struct ReservedRegion {
    pub start: Memory,
    pub end: Memory,
}
