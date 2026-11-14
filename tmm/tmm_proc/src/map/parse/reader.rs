use std::collections::HashMap;

use proc_macro2::Span;
use syn::{parse::Parse, Ident, LitStr, Token};

use crate::map::{Memory, Value};

pub struct FieldReader {
    fields: HashMap<String, (Value, Span)>,
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
