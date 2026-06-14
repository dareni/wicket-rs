/// Helper functions for the proc macros wicket_web_page
/// and wicket_markup_container.
/// On compile, the html files are searched sorted and made
/// statically available for runtime via different strategies
/// configured as features.
use std::{fs, path::PathBuf};

use proc_macro2::TokenStream;
use quote::quote;

use crate::markup::dimension_config::load_dimensions;

/// The Intermediate Representation (IR) of a discovered HTML file.
pub struct DiscoveredMarkup {
    pub style: Option<String>,
    pub variation: Option<String>,
    pub lang: Option<String>,
    pub country: Option<String>,
    pub file_path: String, // e.g., "src/components/MyComponent_fr.html"
}
impl DiscoveredMarkup {
    /// Sort markups by specificity at compile time. Wicket's fallback hierarchy
    /// works top-to-bottom, so the runtime linear scan hits the most specific file first.
    pub fn score(&self) -> u8 {
        let mut score = 0;
        if self.style.is_none() {
            score += 8;
        }
        if self.variation.is_none() {
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

    fn to_match_arms(
        &self,
    ) -> (
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    ) {
        let style = match &self.style {
            Some(s) => quote!(Some(#s)),
            None => quote!(_),
        };
        let variation = match &self.variation {
            Some(v) => quote!(Some(#v)),
            None => quote!(_),
        };
        let lang = match &self.lang {
            Some(l) => quote!(Some(#l)),
            None => quote!(_),
        };
        let country = match &self.country {
            Some(c) => quote!(Some(#c)),
            None => quote!(_),
        };
        (style, variation, lang, country)
    }
}

/// Allow for differing html file search/cache strategies.
pub trait MarkupCodegenStrategy {
    fn generate_codegen(
        &self,
        component_name: &syn::Ident,
        markups: &mut Vec<DiscoveredMarkup>,
    ) -> TokenStream;
}

/// Create match arm code.
pub struct MatchStrategy;
impl MarkupCodegenStrategy for MatchStrategy {
    fn generate_codegen(
        &self,
        component_name: &syn::Ident,
        markups: &mut Vec<DiscoveredMarkup>,
    ) -> TokenStream {
        markups.sort_by_key(|m| m.score());

        // For PRODUCTION create the positive match syntax for each file found and
        // importantly the Borrowed Cow referencing the embedded file.
        #[cfg(not(feature = "dev"))]
        let prod_arms = markups.iter().map(|m| {
            let (style, variation, lang, country) = m.to_match_arms();
            let path = &m.file_path;
            quote! {
                (#style, #variation, #lang, #country) => ::std::borrow::Cow::Borrowed(include_str!(#path)),
            }
        });
        // Generate the PRODUCTION arms. File content baked into the binary.
        #[cfg(not(feature = "dev"))]
        let match_statement = quote! {
            match (style, variation, lang, country) {
                #(#prod_arms)*
                _ => panic!("No matching markup found in prod mode!"),
            }
        };

        //For DEV create the postive match syntax for each file found and
        // by Cow::Owned wrap the file contents on every access.
        #[cfg(feature = "dev")]
        let dev_arms = markups.iter().map(|m| {
            let (style, variation, lang, country) = m.to_match_arms();
            let path = &m.file_path;
            quote! {
                (#style, #variation, #lang, #country) => {
                    // Resolves path relative to workspace root at runtime
                    let content = ::std::fs::read_to_string(#path)
                        .unwrap_or_else(|_| panic!("Failed to hot-reload HTML file at: {}", #path));
                    ::std::borrow::Cow::Owned(content)
                },
            }
        });
        // Generate the DEVELOPMENT arms. Read file content from disk dynamically.
        #[cfg(feature = "dev")]
        let match_statement = quote! {
            match (style, variation, lang, country) {
                #(#dev_arms)*
                _ => panic!("No matching markup found in dev mode!"),
            }
        };

        quote! {
            impl MarkupLookup for #component_name {
                fn lookup_markup(
                    &self,
                    style: Option<&str>,
                    variation: Option<&str>,
                    lang: Option<&str>,
                    country: Option<&str>
                ) -> ::std::borrow::Cow<'static, str> {
                    #match_statement
                }
            }
        }
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

/// Create array definition code.
/// TODO: use rkyv to serialise the array to a binary for inclusion on compile.
#[allow(dead_code)]
pub struct StaticSliceStrategy;
impl MarkupCodegenStrategy for StaticSliceStrategy {
    fn generate_codegen(
        &self,
        component_name: &syn::Ident,
        markups: &mut Vec<DiscoveredMarkup>,
    ) -> TokenStream {
        let array_elements = markups.iter().map(|m| {
            let style = match &m.style {
                Some(s) => quote!(Some(#s)),
                None => quote!(None),
            };
            let variation = match &m.variation {
                Some(v) => quote!(Some(#v)),
                None => quote!(None),
            };
            let lang = match &m.lang {
                Some(l) => quote!(Some(#l)),
                None => quote!(None),
            };
            let country = match &m.country {
                Some(c) => quote!(Some(#c)),
                None => quote!(None),
            };
            let path = &m.file_path;

            // In dev mode, read fresh from disk at runtime but only files existing at
            // compile time will be seen.
            #[cfg(feature = "dev")]
            let markup_str_token = quote! {
                    markup_str: ::std::borrow::Cow::Owned(
                        ::std::fs::read_to_string(#path)
                            .unwrap_or_else(|_| panic!("Failed to hot-reload HTML at: {}", #path))
                    ),
            };
            // In prod mode, bake the file as a string into the binary.
            #[cfg(not(feature = "dev"))]
            let markup_str_token = quote! {
                    markup_str: ::std::borrow::Cow::Borrowed(include_str!(#path)),
            };

            quote! {
                MarkupResource {
                    style: #style,
                    variation: #variation,
                    lang: #lang,
                    country: #country,
                    #markup_str_token
                },
            }
        });

        quote! {
            impl MarkupLookup for #component_name {
                // The shared API method used by your framework.
                fn lookup_markup(
                    &self,
                    style: Option<&str>,
                    variation: Option<&str>,
                    lang: Option<&str>,
                    country: Option<&str>
                ) -> ::std::borrow::Cow<'static, str> {

                    // Define the array  of MarkupResource statically.
                    let resources: &[MarkupResource] = &[
                        #(#array_elements)*
                    ];

                    // The list is pre-sorted by specificity at compile time,
                    // so the first match via standard iteration find() is guaranteed to be
                    // the correct Wicket dimension fallback.
                    let matched = resources.iter().find(|r| {
                        // Match style (if provided, must match; if None, must be None)
                        if r.style != style { return false; }
                        if r.variation != variation { return false; }
                        if r.lang != lang { return false; }
                        if r.country != country { return false; }
                        true
                    });

                    match matched {
                        Some(resource) => resource.markup_str.clone(), // Fast Cow clone
                        None => panic!("No matching markup found for component!"),
                    }
                }
            }
        }
    }
}

/// Main entry referenced by proc macros.
/// component_dir - src relative path of the component for sourcing
///     filesystem html files eg forms/config/entry.
/// component_ident - owner of the html.
/// TokenStream output contains the implementation of the MarkupLookup trait.
pub fn config_static_html(component_dir: &str, component_ident: &syn::Ident) -> TokenStream {
    // Enable the matching strategy.
    #[cfg(feature = "codegen-match")]
    let strategy: Box<dyn MarkupCodegenStrategy> = Box::new(MatchStrategy);
    #[cfg(all(feature = "codegen-static-slice", not(feature = "codegen-match")))]
    let strategy: Box<dyn MarkupCodegenStrategy> = Box::new(StaticSliceStrategy);
    // TODO: create a hashing strategy.
    // TODO: Use rkyv and include_bytes!() to replace the literal array creation.

    // Discover the HTML files in the file system
    let mut discovered_markups =
        discover_files(component_ident.to_string().as_mut_str(), component_dir);

    // Generate the strategy-specific code
    let generated_codegen = strategy.generate_codegen(component_ident, &mut discovered_markups);

    quote! {
        #generated_codegen
    }
}

pub fn discover_files(component_dir: &str, component_name: &str) -> Vec<DiscoveredMarkup> {
    let mut discovered = Vec::new();

    let mut search_path = PathBuf::from(component_dir);
    search_path.push("src");
    search_path.push("components");

    let entries = fs::read_dir(&search_path).ok().into_iter().flatten();
    let valid_dimensions = load_dimensions();

    for entry in entries.flatten() {
        let path = entry.path();
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
                lang = Some(segment.to_string());
            } else if segment.len() == 2 && segment.chars().all(|c| c.is_ascii_uppercase()) {
                // Strict ISO 3166-1 alpha-2 country code check (e.g., "CA", "US")
                country = Some(segment.to_string());
            } else {
                // Crucial Rust Feature: Compile-time panic on invalid user files!
                panic!(
                    "Invalid Wicket markup file found: '{}'. The segment '{}' does not match any valid style, variation, language, or country rule.",
                    file_name, segment
                );
            }
        }

        discovered.push(DiscoveredMarkup {
            style,
            variation,
            lang,
            country,
            file_path: path.to_string_lossy().into_owned(),
        });
    }

    // Sort by specificity score so drop-in strategies match correctly
    discovered.sort_by_key(|m| m.score());
    discovered
}
