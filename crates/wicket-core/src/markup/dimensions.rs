use wicket_macro::load_html_dimensions;
use wicket_macro_support::get_string_index;

#[derive(Default)]
pub struct ValidHtmlDimensions {
    pub style: Option<Vec<String>>,
    pub variation: Option<Vec<String>>,
    pub lang: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
}

impl ValidHtmlDimensions {
    pub fn get_style_index<S: AsRef<str>>(&self, style: S) -> Option<u8> {
        get_string_index(style, get_valid_html_dimensions().style.as_deref())
    }

    pub fn get_variation_index<V: AsRef<str>>(&self, variation: V) -> Option<u8> {
        get_string_index(variation, get_valid_html_dimensions().variation.as_deref())
    }

    pub fn get_lang_index<L: AsRef<str>>(&self, lang: L) -> Option<u8> {
        get_string_index(lang, get_valid_html_dimensions().lang.as_deref())
    }

    pub fn get_country_index<C: AsRef<str>>(&self, country: C) -> Option<u8> {
        get_string_index(country, get_valid_html_dimensions().country.as_deref())
    }
}

// Create a static ValidHtmlDimensions struct from the toml config file.
load_html_dimensions!();

pub fn get_valid_html_dimensions() -> &'static ValidHtmlDimensions {
    &VALID_HTML_DIMENSIONS
}

#[cfg(test)]


    }
}
