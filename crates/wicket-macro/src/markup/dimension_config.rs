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
pub struct TomlDimensions {
    pub style: Option<Vec<String>>,
    pub variation: Option<Vec<String>>,
    pub lang: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
}

static CONFIG_FILE_NAME: &str = "html_dimensions.toml";

pub fn run_load_html_dimensions(_input: TokenStream) -> TokenStream {
    let config = load_dimensions();

    let style_tokens = helper_opt_vec(config.style);
    let variation_tokens = helper_opt_vec(config.variation);
    // Strict ISO 639-1 alpha-2 language code check (e.g., "fr", "en")
    let valid_lang = config.lang.as_ref().is_none_or(|langs| {
        langs
            .iter()
            .all(|lang| lang.len() == 2 && lang.chars().all(|c| c.is_lowercase()))
    });
    assert!(
        valid_lang,
        "Language codes are 2 char in length and lowercase."
    );
    let lang_tokens = helper_opt_vec(config.lang);
    // Strict ISO 3166-1 alpha-2 country code check (e.g., "CA", "US")
    let valid_country = config.country.as_ref().is_none_or(|ctrys| {
        ctrys
            .iter()
            .all(|ctry| ctry.len() == 2 && ctry.chars().all(|c| c.is_uppercase()))
    });
    assert!(
        valid_country,
        "Country codes are 2 char in length and uppercase."
    );

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

pub fn load_dimensions() -> TomlDimensions {
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
    if path.exists() {
        let content = fs::read_to_string(path).expect("Failed to read html_dimensions.toml");
        toml::from_str::<TomlDimensions>(&content).unwrap_or_else(|e| {
            panic!("Invalid TOML formatting in html_dimensions.toml: {}", e);
        })
    } else {
        TomlDimensions::default()
    }
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
