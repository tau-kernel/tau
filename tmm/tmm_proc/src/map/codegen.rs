use std::sync::Arc;

use crate::map::{parse, Memory, Region, RegionInfo};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{Attribute, Ident, LitStr};

#[derive(Clone, Debug)]
struct ResponseStatic {
    name: Ident,
}

impl ToTokens for ResponseStatic {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = &self.name;
        tokens.extend(quote! {
            static #name: Once<MemoryRegion> = Once::new();
        });
    }
}

#[derive(Clone, Debug)]
struct RequestConst {
    const_name: Ident,
    pretty_name: LitStr,
    size: Memory,
    align: Memory,
}

impl ToTokens for RequestConst {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let RequestConst {
            const_name,
            pretty_name,
            size,
            align,
        } = &self;

        tokens.extend(quote! {
            const #const_name: RegionRequest = RegionRequest {
                name: #pretty_name,
                size: #size,
                alignment: ::core::mem::Alignment::new(#align).expect(concat!(
                        "tmm::memory_map: ", stringify!(#align), " is not a power of two"))
            };
        });
    }
}

#[derive(Clone, Debug)]
struct ManagerInitStmt<'a, 'b> {
    manager: Ident,
    module: Ident,
    rng: Ident,
    request: &'a RequestConst,
    response: &'b ResponseStatic,
}

impl<'a, 'b> ToTokens for ManagerInitStmt<'a, 'b> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ManagerInitStmt {
            manager,
            module,
            rng,
            request,
            response,
        } = &self;
        let request_name = &request.const_name;
        let request_pretty_name = &request.pretty_name;
        let response_name = &response.name;
        // Commits the given memory region to the backing, ensuring that it does not overlap with any existing regions.
        // pub fn commit(&mut self, region: MemoryRegion)

        // Fulfils the given range request and commits it to the backing.
        // pub fn fulfill_and_commit<R: Rng>(
        //     &mut self,
        //     r: &mut R,
        //     req: RegionRequest,
        // ) -> Option<MemoryRegion>

        let request_path = quote! {
            #module::#request_name
        };
        let response_path = quote! {
            #module::#response_name
        };
        tokens.extend(quote! {{
                let mut val = Some(#manager.fulfill_and_commit(#rng, #request_path)
                    .expect(concat!("failed to fulfill kernel range ", #request_pretty_name)));
                ::cake::log::info!(target: "memory_map", "mapped range {} to {:?}", #request_pretty_name, val);
                #response_path.call_once(|| val.take().unwrap());
                assert!(val.is_none(), "Response already init!");
        }});
    }
}

#[derive(Clone, Debug)]
struct ReservedConst {
    const_name: Ident,
    pretty_name: LitStr,
    start: Memory,
    end: Memory,
}

impl ReservedConst {
    fn from_parse(ifo: &parse::RegionInfo, reserved: &parse::ReservedRegion) -> Self {
        ReservedConst {
            const_name: format_ident!("{}_RESERVED", &ifo.ident),
            pretty_name: ifo.pretty_name.clone(),
            start: reserved.start.clone(),
            end: reserved.end.clone(),
        }
    }
    fn reserve(&self, tokens: &mut proc_macro2::TokenStream, manager: Ident) {
        let const_name = &self.const_name;
        tokens.extend(quote! {
            #manager.commit(#const_name);
        });
    }
}

impl ToTokens for ReservedConst {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ReservedConst {
            const_name,
            pretty_name,
            start,
            end,
        } = &self;

        tokens.extend(quote! {
            const #const_name: MemoryRegion = MemoryRegion::new(
                VirtRange::new(VirtAddr::new(#start), VirtAddr::new(#end)),
                #pretty_name
            );
        });
    }
}

struct Request {
    module: Ident,
    attribs: Vec<Attribute>,
    request_const: RequestConst,
    response_static: ResponseStatic,
}

impl Request {
    fn from_parser(info: &parse::RegionInfo, data: &parse::RequestedRegion) -> Self {
        let lowercase = Ident::new(&info.ident.to_string().to_lowercase(), info.ident.span());
        Self {
            module: lowercase,
            attribs: info.attribs.clone(),
            request_const: RequestConst {
                const_name: format_ident!("{}_REQUEST", &info.ident),
                pretty_name: info.pretty_name.clone(),
                size: data.size.clone(),
                align: data.align.clone(),
            },
            response_static: ResponseStatic {
                name: format_ident!("{}_RESPONSE", &info.ident),
            },
        }
    }

    fn module(&self, ts: &mut TokenStream) {
        let Request {
            attribs,
            module,
            request_const,
            response_static,
        } = &self;
        ts.extend(quote! {
            mod #module {
                use super::*;
                pub(super) #request_const
                pub(super) #response_static
            }
        });
    }

    fn request(&self, ts: &mut TokenStream, manager: Ident, rng: Ident) {
        let req_stmt = ManagerInitStmt {
            manager,
            module: self.module.clone(),
            rng: rng,
            request: &self.request_const,
            response: &self.response_static,
        };

        ts.extend(quote! {
            #req_stmt
        });
    }

    fn getter(&self, ts: &mut TokenStream) {
        let getter_fn = Ident::new(
            &format!("get_{}", self.module.to_string()),
            self.module.span(),
        );
        let response = &self.response_static.name;
        let attribs = &self.attribs;
        let module = &self.module;
        let resp_path = quote! {#module::#response};
        ts.extend(quote! {
            #(#attribs)*
            pub fn #getter_fn() -> &'static MemoryRegion {
                #resp_path.get().expect(concat!(stringify!(#getter_fn), ": Failed to get region, is it uninit?"))
            }
        });
    }
}

struct MemoryMap {
    requests: Vec<Request>,
    reservations: Vec<ReservedConst>,
}

impl MemoryMap {
    pub fn from_parse(regions: &[parse::RegionInfo]) -> Self {
        let mut requests = vec![];
        let mut reservations = vec![];
        for region in regions {
            match &region.region {
                Region::Request(requested_region) => {
                    requests.push(Request::from_parser(region, &requested_region))
                }
                Region::Reserved(reserved_region) => {
                    reservations.push(ReservedConst::from_parse(region, &reserved_region))
                }
            }
        }

        Self {
            requests,
            reservations,
        }
    }
}

impl ToTokens for MemoryMap {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut modules = TokenStream::new();
        let mut getters = TokenStream::new();
        let mut requests = TokenStream::new();

        let builder_ident = Ident::new("builder", Span::call_site());
        let rng = Ident::new("rng", Span::call_site());
        for region in &self.requests {
            region.module(&mut modules);
            region.getter(&mut getters);
            region.request(&mut requests, builder_ident.clone(), rng.clone());
        }

        let mut reservation_consts = TokenStream::new();
        let mut reservation_stmts = TokenStream::new();
        for reservation in &self.reservations {
            reservation.to_tokens(&mut reservation_consts);
            reservation.reserve(&mut reservation_stmts, builder_ident.clone());
        }

        tokens.extend(quote! {
            /// The memory map for the kernel.
            pub mod memory_map {
                use ::cake::Once;
                use ::alloc::alloc::Allocator;
                use ::tmm::{region::{MemoryRegion, RegionRequest}, paging::primitives::{VirtRange, VirtAddr, address::Address}};
            #modules
            #getters

            #reservation_consts

            /// Inits all of the regions with the given region manager.
            pub fn init_with_manager<A: Allocator + Clone, R: ::cake::rand::Rng, const INLINE_CAP: usize>
                    (#builder_ident: &mut ::tmm::region::RegionManager<A, INLINE_CAP>, rng: &mut R) {
                        #reservation_stmts

                        #requests

            }

            }
        });
    }
}

pub fn impl_map(map: parse::KernelMap) -> TokenStream {
    let map = MemoryMap::from_parse(&map.regions);
    map.into_token_stream()
}
