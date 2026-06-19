/// Helper functions for the proc macros wicket_web_page
/// and wicket_markup_container.
/// On compile, the html files are searched sorted and made
/// statically available for runtime via different strategies
/// configured as features.
use std::{collections::HashMap, fs, path::PathBuf};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use wicket_macro_support::get_string_index;

use crate::markup::dimension_config::{TomlDimensions, load_dimensions};

/// The Intermediate Representation (IR) of a discovered HTML file.
/// In Apache Wicket, HTML files are resolved using the ResourceStreamLocator class,
/// which combines the component's variation, the session's style, and the thread's locale.
pub struct DiscoveredMarkup {
    pub variation: Option<String>,
    pub style: Option<String>,
    pub lang: Option<String>,
    pub country: Option<String>,
    pub file_path: String, // e.g., "src/components/MyComponent_fr.html"
}
impl DiscoveredMarkup {
    /// Sort markups by specificity at compile time. Wicket's fallback hierarchy
    /// works top-to-bottom, so the runtime linear scan hits the most specific file first.
    pub fn score(&self) -> u8 {
        let mut score = 0;
        if self.variation.is_none() {
            score += 8;
        }
        if self.style.is_none() {
            score += 4;
        }
        if self.lang.is_none() {
            score += 2;
        }
        if self.country.is_none() {
            score += 1;
        }
        score
    }
}

pub fn get_crate_root(name: &str) -> TokenStream {
    let Ok(crate_search) = crate_name(name) else {
        panic!(
            "{} wicket-core must be in Cargo.toml. {:?}",
            name,
            crate_name(name).err()
        );
    };

    match crate_search {
        // Case A: The macro is being called INSIDE `wicket-core`
        FoundCrate::Itself => quote! { crate },

        // Case B: The macro is being called from another crate using `::`
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, proc_macro::Span::call_site().into());
            quote! { ::#ident }
        }
    }
}

const MARKUP_RESOURCE_ARRAY_CONST_PREFIX: &str = "_MARKUP_RESOURCE_VEC_";

/// Create array definition code.
fn generate_codegen(component_name: &syn::Ident, markups: &mut [DiscoveredMarkup]) -> TokenStream {
    let crate_root = get_crate_root("wicket-core");
    let valid_dimensions = load_dimensions();
    let array_elements = markups.iter().map(|m| {
        let style = match &m.style {
            Some(s) => {
                let style_indx = get_string_index(s, valid_dimensions.style.as_deref());
                quote!(Some(#style_indx))
            }
            None => quote!(None),
        };
        let variation = match &m.variation {
            Some(v) => {
                let variation_indx = get_string_index(v, valid_dimensions.variation.as_deref());
                quote!(Some(#variation_indx))
            }
            None => quote!(None),
        };
        let lang = match &m.lang {
            Some(l) => {
                let lang_indx = get_string_index(l, valid_dimensions.lang.as_deref());
                quote!(Some(#lang_indx))
            }
            None => quote!(None),
        };
        let country = match &m.country {
            Some(c) => {
                let country_indx = get_string_index(c, valid_dimensions.country.as_deref());
                quote!(Some(#country_indx))
            }
            None => quote!(None),
        };
        let path = &m.file_path;

        // In dev mode, read fresh from disk at runtime but only files existing at
        // compile time will be seen.
        let markup_str_token = if cfg!(feature = "dev") {
            quote! {
                    markup_str: ::std::borrow::Cow::Owned(
                        ::std::fs::read_to_string(#path)
                            .unwrap_or_else(|_| panic!("Failed to hot-reload HTML at: {}", #path))
                    ),
            }
        } else {
            // In prod mode, bake the file as a string into the binary.
            quote! {
                    markup_str: ::std::borrow::Cow::Borrowed(include_str!(#path)),
            }
        };

        // TODO: Use rkyv and include_bytes!() to replace the literal array creation.
        quote! {
            #crate_root::markup::MarkupResource {
                style: #style,
                variation: #variation,
                lang: #lang,
                country: #country,
                #markup_str_token
            },
        }
    });

    let markup_resource_vec_name = quote::format_ident!(
        "{}{}",
        MARKUP_RESOURCE_ARRAY_CONST_PREFIX,
        component_name.to_string().to_uppercase()
    );
    let resource_count = markups.len();

    let static_array_declaration = if cfg!(feature = "dev") {
        quote! {
        static #markup_resource_vec_name : ::std::sync::LazyLock
            <[#crate_root::markup::MarkupResource; #resource_count]> = ::std::sync::LazyLock::new(||{[
                    #(#array_elements)*
        ]});
        }
    } else {
        quote! {
        static #markup_resource_vec_name : [#crate_root::markup::MarkupResource; #resource_count] = [
                    #(#array_elements)*
        ];
        }
    };

    quote! {

        #static_array_declaration

        impl #crate_root::components::MarkupLookup for #component_name {
            fn lookup_markup(
                &self,
                style: Option<u8>,
                variation: Option<u8>,
                lang: Option<u8>,
                country: Option<u8>
            ) -> Option<&#crate_root::markup::MarkupResource> {

                // The list is pre-sorted by specificity at compile time,
                // so the first match via standard iteration find() is guaranteed to be
                // the correct Wicket dimension fallback.
                #markup_resource_vec_name.iter().find(|r| {
                    // Match style (if provided, must match; if None, must be None)
                    if r.style != style { return false; }
                    if r.variation != variation { return false; }
                    if r.lang != lang { return false; }
                    if r.country != country { return false; }
                    true
                })
            }
        }
    }
}

/// Main entry referenced by proc macros.
/// component_dir - src relative path of the component for sourcing
///     filesystem html files eg forms/config/entry.
/// component_ident - owner of the html.
/// TokenStream output contains the implementation of the MarkupLookup trait.
pub fn config_static_html(component_dir: PathBuf, component_ident: &syn::Ident) -> TokenStream {
    // Discover the HTML files in the file system
    let mut discovered_markups =
        discover_files(component_dir, component_ident.to_string().as_mut_str());

    let generated_codegen = generate_codegen(component_ident, &mut discovered_markups);

    quote! {
        #generated_codegen
    }
}

pub fn discover_files(component_dir: PathBuf, component_name: &str) -> Vec<DiscoveredMarkup> {
    if !component_dir.exists() {
        panic!(
            "HTML file path does not exist: {}",
            component_dir.to_string_lossy()
        );
    }

    let entries = fs::read_dir(&component_dir).ok().into_iter().flatten();
    let mut file_list: Vec<PathBuf> = vec![];
    for entry in entries.flatten() {
        file_list.push(entry.path());
    }
    let valid_dimensions = load_dimensions();
    let discovered = build_discovered_markup_vec(&file_list, component_name, &valid_dimensions);

    if discovered.is_empty() {
        let entries = fs::read_dir(&component_dir).ok().into_iter().flatten();
        let mut file_csv = String::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy();
            if file_csv.is_empty() {
                file_csv.push_str(&file_name)
            } else {
                file_csv.push(',');
                file_csv.push_str(&file_name)
            }
        }
        panic!(
            "No matching html files found in directory:'{}'. Files found:'{}'. No files matching:'{}'. ",
            component_dir.to_string_lossy(),
            file_csv,
            component_name
        )
    }
    discovered
}

pub fn build_discovered_markup_vec(
    files: &[PathBuf],
    component_name: &str,
    valid_dimensions: &TomlDimensions,
) -> Vec<DiscoveredMarkup> {
    type DimensionsTup = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let mut discovered_map: HashMap<DimensionsTup, DiscoveredMarkup> = HashMap::new();

    for path in files {
        let file_name = path.file_name().unwrap().to_string_lossy();

        // Must be an HTML file and belong to this component
        if !file_name.starts_with(component_name) || !file_name.ends_with(".html") {
            continue;
        }

        // Stripping the ".html" extension before splitting
        let clean_name = &file_name[..file_name.len() - 5];

        // Split by period. Example: "MyComponent.dark.fr.CA" -> ["MyComponent", "dark", "fr", "CA"]
        let segments: Vec<&str> = clean_name.split('.').collect();

        // The first element is always the base component name
        if segments.is_empty() || segments[0] != component_name {
            continue;
        }

        // Mutable modifiers we will populate as we inspect segments [1..]
        let mut style = None;
        let mut variation = None;
        let mut lang = None;
        let mut country = None;

        // Categorize the dimensions.
        for &segment in &segments[1..] {
            // if VALID_STYLES.contains(&segment) {
            if valid_dimensions
                .style
                .as_ref()
                .is_some_and(|styles| styles.iter().any(|s| s == segment))
            {
                style = Some(segment.to_string());
            } else if valid_dimensions
                .variation
                .as_ref()
                .is_some_and(|vari| vari.iter().any(|v| v == segment))
            {
                variation = Some(segment.to_string());
            } else if valid_dimensions
                .lang
                .as_ref()
                .is_some_and(|lang| lang.iter().any(|l| l == segment))
            {
                //ISO 639-2/3 allow for 2-3 chars plus extended language subtags.
                lang = Some(segment.to_string());
            } else if segment.len() == 2 && segment.chars().all(|c| c.is_ascii_uppercase()) {
                // Strict ISO 3166-1 alpha-2 country code check (e.g., "CA", "US")
                country = Some(segment.to_string());
            } else {
                // Compile-time panic on invalid user files!
                panic!(
                    "Invalid Wicket markup file found: '{}'. The segment '{}' does not match any valid style, variation, language, or country rule. Note: country codes are strict ISO 3166-1 eg AU,GB,US",
                    file_name, segment
                );
            }
        }

        //Use a map to check for mixed ordering of dimensions eg ...EN.au.html and ...AU.en.html
        let dup = discovered_map.insert(
            (
                style.clone(),
                variation.clone(),
                lang.clone(),
                country.clone(),
            ),
            DiscoveredMarkup {
                style,
                variation,
                lang,
                country,
                file_path: path.to_string_lossy().into_owned(),
            },
        );
        match dup {
            None => (),
            Some(dup) => panic!(
                "Error duplicate Html file {} with dimensions style:{} variation:{} lang:{} country:{}",
                component_name,
                dup.style.unwrap_or("None".to_string()),
                dup.variation.unwrap_or("None".to_string()),
                dup.lang.unwrap_or("None".to_string()),
                dup.country.unwrap_or("None".to_string())
            ),
        }
    }

    let mut discovered: Vec<DiscoveredMarkup> = discovered_map.into_values().collect();

    // Sort by specificity score so iter.find() returns the correct dimension first.
    discovered.sort_by_key(|m| m.score());
    discovered
}

#[cfg(test)]
mod test {

    use std::{path::PathBuf, str::FromStr};

    use crate::markup::{dimension_config::TomlDimensions, discovery::build_discovered_markup_vec};

    #[test]
    #[should_panic(expected = "Invalid Wicket markup file found: 'MainPage.au.html'.")]
    pub fn test_build_discovered_markup_vec_invalid_country() {
        let files = [PathBuf::from_str("resources/MainPage.au.html").unwrap()];
        let component_name = "MainPage";
        let valid_dimensions = TomlDimensions {
            style: None,
            variation: None,
            lang: None,
            country: Some(vec!["au".to_string()]),
        };

        let _ = build_discovered_markup_vec(&files, component_name, &valid_dimensions);
    }

    #[test]
    #[should_panic(expected = "Error duplicate Html file MainPage")]
    pub fn test_build_discovered_markup_vec_dups() {
        let valid_dimensions = TomlDimensions {
            style: Some(vec!["light".to_string()]),
            variation: Some(vec!["mobile".to_string()]),
            lang: Some(vec!["en".to_string()]),
            country: Some(vec!["AU".to_string()]),
        };
        let component_name = "MainPage";
        let files = [
            PathBuf::from_str("resources/MainPage.en.AU.html").unwrap(),
            PathBuf::from_str("resources/MainPage.AU.en.html").unwrap(),
        ];
        let _ = build_discovered_markup_vec(&files, component_name, &valid_dimensions);
    }

    use super::DiscoveredMarkup;
    impl DiscoveredMarkup {
        pub fn dimensions_match(
            &self,
            variation: Option<&str>,
            style: Option<&str>,
            lang: Option<&str>,
            country: Option<&str>,
        ) -> bool {
            // let b_style = &self.variation.is_some_and(|s | variation.is_some_and(f))
            self.variation.as_deref() == variation
                && self.style.as_deref() == style
                && self.lang.as_deref() == lang
                && self.country.as_deref() == country
        }
    }

    #[test]
    pub fn test_build_discovered_markup_vec() {
        let valid_dimensions = TomlDimensions {
            style: Some(vec!["light".to_string()]),
            variation: Some(vec!["mobile".to_string()]),
            lang: Some(vec!["en".to_string()]),
            country: Some(vec!["AU".to_string()]),
        };
        let component_name = "MainPage";
        let files = [PathBuf::from_str("resources/MainPage.html").unwrap()];
        let markup_vec = build_discovered_markup_vec(&files, component_name, &valid_dimensions);
        assert!(markup_vec.len() == 1);
        let mu = &markup_vec[0];
        assert!(
            mu.style.is_none()
                && mu.variation.is_none()
                && mu.lang.is_none()
                && mu.country.is_none()
        );
        assert!(!mu.file_path.is_empty());
        assert!(mu.file_path == "resources/MainPage.html");

        let files = [PathBuf::from_str("resources/MainPage.AU.html").unwrap()];
        let _markup_vec = build_discovered_markup_vec(&files, component_name, &valid_dimensions);

        let files = [
            PathBuf::from_str("resources/MainPage.en.html").unwrap(),
            //make sure this one is ommitted
            PathBuf::from_str("resources/MainPage1.html").unwrap(),
            PathBuf::from_str("resources/MainPage.html").unwrap(),
            PathBuf::from_str("resources/MainPage.AU.html").unwrap(),
            PathBuf::from_str("resources/MainPage.AU.en.html").unwrap(),
            PathBuf::from_str("resources/MainPage.mobile.html").unwrap(),
            PathBuf::from_str("resources/MainPage.mobile.en.html").unwrap(),
        ];

        let markup_vec = build_discovered_markup_vec(&files, component_name, &valid_dimensions);
        // Verify DiscoveredMarkup order.
        assert!(&markup_vec[0].dimensions_match(Some("mobile"), None, Some("en"), None));
        assert!(&markup_vec[1].dimensions_match(Some("mobile"), None, None, None));
        assert!(&markup_vec[2].dimensions_match(None, None, Some("en"), Some("AU")));
        assert!(&markup_vec[3].dimensions_match(None, None, Some("en"), None));
        assert!(&markup_vec[4].dimensions_match(None, None, None, Some("AU")));
        assert!(&markup_vec[5].dimensions_match(None, None, None, None));
    }
}
