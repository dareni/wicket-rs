#[derive(Default)]
pub struct ValidHtmlDimensions {
    pub style: Option<Vec<String>>,
    pub variation: Option<Vec<String>>,
    pub lang: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
}

/// Create a static ValidHtmlDimensions struct from the toml config file.
#[cfg(not(test))]
pub mod dimension_provider {
    use super::ValidHtmlDimensions;
    use wicket_macro::load_html_dimensions;

    load_html_dimensions!();

    pub fn get_valid_html_dimensions() -> &'static ValidHtmlDimensions {
        &VALID_HTML_DIMENSIONS
    }
}

#[cfg(test)]
pub mod dimension_provider {
    use super::ValidHtmlDimensions;
    use std::sync::OnceLock;

    static VALID_HTML_DIMENSIONS: OnceLock<ValidHtmlDimensions> = std::sync::OnceLock::new();

    pub fn get_valid_html_dimensions() -> &'static ValidHtmlDimensions {
        VALID_HTML_DIMENSIONS.get_or_init(ValidHtmlDimensions::default)
    }
}
