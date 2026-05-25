//! Load optional configuration providing runtime HTML rendering customization.
//!
//! This module handles customization based on the dimensions of style, variation,
//! language, and country. Valid dimensions are configured via a TOML file,
//! typically named [`html_dimensions.toml`](CONFIG_FILE_NAME). Creation of this
//! file is the only configuration necessary.

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct TomlDimensions {
    style: Option<Vec<String>>,
    variation: Option<Vec<String>>,
    lang: Option<Vec<String>>,
    country: Option<Vec<String>>,
}

static CONFIG_FILE_NAME: &str = "html_dimensions.toml";

pub fn run_load_html_dimensions(_input: TokenStream) -> TokenStream {
    let mut path = {
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        } else {
            let mut p = env::current_exe().expect("Failed to get exe path");
            p.pop();
            p
        }
    };
    path.push(CONFIG_FILE_NAME);

    // Read and parse file, or fallback to default (None values)
    let config = if path.exists() {
        let content = fs::read_to_string(path).expect("Failed to read html_dimensions.toml");
        toml::from_str::<TomlDimensions>(&content).unwrap_or_else(|e| {
            panic!("Invalid TOML formatting in html_dimensions.toml: {}", e);
        })
    } else {
        TomlDimensions::default()
    };

    let style_tokens = helper_opt_vec(config.style);
    let variation_tokens = helper_opt_vec(config.variation);
    let lang_tokens = helper_opt_vec(config.lang);
    let country_tokens = helper_opt_vec(config.country);

    let expanded = quote! {

        pub static VALID_HTML_DIMENSIONS: ValidHtmlDimensions = ValidHtmlDimensions {
            style: #style_tokens,
            variation: #variation_tokens,
            lang: #lang_tokens,
            country: #country_tokens,
        };

    };

    TokenStream::from(expanded)
}

fn helper_opt_vec(opt: Option<Vec<String>>) -> proc_macro2::TokenStream {
    match opt {
        Some(vec) => {
            let items = vec.iter().map(|s| quote! { #s });
            // Macro repetition  #( for expanding the list of items.
            quote! { Some(vec![#(#items),*]) }
        }
        None => quote! { None },
    }
}
