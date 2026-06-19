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
pub mod test {
    use wicket_macro::wicket_page;
    use wicket_request::request::mapper::parameter::PageParameters;

    use crate::components::{FromPageParameters, MarkupContainer, WebPage};
    #[test]
    pub fn test_dimension_html_load() {
        #[wicket_page("tests/resources/html/markup/dimensions")]
        pub struct DimensionsTestPage {}
        impl FromPageParameters for DimensionsTestPage {
            fn from_page_params(_page_params: Option<PageParameters>) -> Box<dyn WebPage> {
                Box::new(Self {})
            }
        }
        impl WebPage for DimensionsTestPage {}

        impl MarkupContainer for DimensionsTestPage {
            fn render_component(
                &self,
                _id: crate::components::ComponentId,
                _response: &mut crate::request::Response,
            ) -> std::io::Result<crate::request::cycle::RedirectAction> {
                unimplemented!()
            }
        }

        let page = DimensionsTestPage::from_page_params(None);
        let markup_resource = page.lookup_markup(None, None, None, None);
        assert!(markup_resource.is_some());
        let markup_resource = markup_resource.unwrap();
        let html = &markup_resource.markup_str;
        assert!(
            html.trim_end() == "<html></html>",
            "Because: '<html></html>' != '{}'",
            html.trim_end()
        );
        assert!(_MARKUP_RESOURCE_VEC_DIMENSIONSTESTPAGE.len() == 1);
        let page = page.lookup_markup(Some(1), None, None, None);
        assert!(page.is_none());
    }
}
