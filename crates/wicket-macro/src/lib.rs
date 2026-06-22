mod markup;

use std::path::PathBuf;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, LitStr, parse_macro_input};

use crate::markup::{
    dimension_config::run_load_html_dimensions,
    discovery::{config_static_html, get_crate_root},
};
use wicket_macro_support::hash_string;

/// Create a static ValidHtmlDimensions struct from the toml config file.
/// See [::wicket-core::markup::dimensions::dimension_provider]
#[proc_macro]
pub fn load_html_dimensions(input: TokenStream) -> TokenStream {
    run_load_html_dimensions(input)
}

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
    let crate_root = get_crate_root("wicket-core");
    let expanded: proc_macro2::TokenStream = quote! {
        impl #crate_root::markup::loader::MarkupResourceLocationUtil for #name {
            fn get_component_path(&self) -> &'static str {
                module_path!()
            }

            fn get_component_name(&self) -> &'static str {
                stringify!(#name)
            }

            fn get_markup_type(&self) -> &'static str {
               ::wicket_util::constants::file_ext::HTML
            }

        }
    };

    expanded
}

#[proc_macro_attribute]
pub fn wicket_markup_container(attribs: TokenStream, item: TokenStream) -> TokenStream {
    let item_input = parse_macro_input!(item as DeriveInput);
    let name = &item_input.ident;
    let comp_dir_attrib = parse_macro_input!(attribs as LitStr);

    let path = std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
    let relative_component_dir: String = comp_dir_attrib.value();
    let mut component_dir = PathBuf::from(path);
    component_dir.push(relative_component_dir);

    config_static_html(component_dir, name).into()
}

const PAGE_ID_CONST_PREFIX: &str = "WICKETPAGEID_";

/// Note: the target struct must implement FromPageParameters.
/// The only way to obtain the component html path at compile time is via a static macro argument.
///
/// example:
///
/// #[wicket_page("tests/resources/html/account")]
/// struct AccountMainPageTest{}
///
/// or:
///
/// #[wicket_page("src/account")]
/// struct AccountMainPage{}
///
#[proc_macro_attribute]
pub fn wicket_page(attribs: TokenStream, item: TokenStream) -> TokenStream {
    let comp_dir_attrib = parse_macro_input!(attribs as LitStr);
    let relative_component_dir: String = comp_dir_attrib.value();

    let item_input = parse_macro_input!(item as DeriveInput);
    let name = &item_input.ident;

    let path = std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");

    let mut component_dir = PathBuf::from(path);
    component_dir.push(relative_component_dir);

    let html_data = config_static_html(component_dir, name);

    let const_name = quote::format_ident!(
        "{}{}",
        PAGE_ID_CONST_PREFIX,
        name.to_string().to_uppercase()
    );
    let location_impl = get_impl_markup_resource_location_util(name);
    let id = hash_string(name.to_string().as_str());

    let crate_root = get_crate_root("wicket-core");
    let expanded = quote! {

    #[derive(Clone)]
    #item_input

    #location_impl

    pub static #const_name : #crate_root::components::MarkupType = #crate_root::components::MarkupType {
        id: #id,
        name: stringify!(#name),
    };

     impl #crate_root::components::MarkupIdentifier for #name {
         fn get_markup_identity(&self) -> &'static #crate_root::components::MarkupType {
             &#const_name
         }
     }

    #html_data

    inventory::submit! {
        #crate_root::session::page_factory::PageEntry {
            id: &#const_name,
            constructor: |params| {
                #crate_root::session::page_factory::PageEntry::from_page_params::<#name>(params)
            }
        }
    }

    };

    TokenStream::from(expanded)
}
