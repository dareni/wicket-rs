use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// The most reliable technique for obtaining module path and struct name.
#[proc_macro_derive(MarkupResourcePath)]
pub fn derive_markup_resource_path(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Build the output tokens using the quote! macro
    // will expand to the module where the struct is defined,
    // not where the macro lives.

    let expanded = quote! {
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

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn page_factory_config(_attribs: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let expanded = quote! {

    #input

    inventory::submit! {
        PageEntry {
            name: stringify!(#name),
            constructor: |params| {
                Box::new(#name::create_from_params(params))
            }
        }
    }

    };

    TokenStream::from(expanded)
}
