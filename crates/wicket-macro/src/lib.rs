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
    let expanded = quote! {
        impl MarkupResourceProvider for #name {
            fn get_component_path(&self) -> &'static str {
                // will expand to the module where the struct is defined,
                // not where the macro lives.
                module_path!()
            }

            fn get_component_name(&self) -> &'static str {
                stringify!(#name)
            }
        }
    };

    TokenStream::from(expanded)
}
