use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

use wicket_macro_support::hash_string;

/// The most reliable technique for obtaining module path and struct name.
#[proc_macro_derive(MarkupResourcePath)]
pub fn derive_markup_resource_path(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Build the output tokens using the quote! macro
    // will expand to the module where the struct is defined,
    // not where the macro lives.
    TokenStream::from(get_impl_markup_resource_location_util(&name))
}

fn get_impl_markup_resource_location_util(name: &Ident) -> proc_macro2::TokenStream {
    let expanded: proc_macro2::TokenStream = quote! {
        impl MarkupResourceLocationUtil for #name {
            fn get_component_path(&self) -> &'static str {
                module_path!()
            }

            fn get_component_name(&self) -> &'static str {
                stringify!(#name)
            }

            fn get_markup_type(&self) -> &'static str {
               file_ext::HTML
            }

        }
    };

    expanded
}

const PAGE_ID_CONST_PREFIX: &str = "WICKETPAGEID_";

#[proc_macro_attribute]
pub fn wicket_page(_attribs: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let const_name = quote::format_ident!(
        "{}{}",
        PAGE_ID_CONST_PREFIX,
        name.to_string().to_uppercase()
    );
    let location_impl = get_impl_markup_resource_location_util(name);
    let id = hash_string(name.to_string().as_str());
    let expanded = quote! {

    #[derive(Clone)]
    #input

    #location_impl

    pub static #const_name : PageType = PageType {
        id: #id,
        name: stringify!(#name),
    };

     impl PageIdentifier for #name {
         fn get_page_identity(&self) -> &'static PageType {
             &#const_name
         }
     }

    inventory::submit! {
        PageEntry {
            id: &#const_name,
            constructor: |params| {
                Box::new(#name::create_from_params(params))
            }
        }
    }

    };

    TokenStream::from(expanded)
}
