use std::{collections::HashMap, sync::OnceLock};

use wicket_request::request::mapper::parameter::PageParameters;

use crate::components::WebPage;

inventory::collect!(PageEntry);

type WebPageConstructor = fn(params: Option<PageParameters>) -> Box<dyn WebPage>;
static PAGE_FACTORY: OnceLock<HashMap<&'static str, &WebPageConstructor>> = OnceLock::new();

pub fn construct_page(name: &str, params: Option<PageParameters>) -> Box<dyn WebPage> {
    let page_inventory = PAGE_FACTORY.get_or_init(|| {
        let mut page_map: HashMap<&'static str, &WebPageConstructor> = HashMap::new();
        for entry in inventory::iter::<PageEntry> {
            page_map.insert(entry.name, &entry.constructor);
        }
        page_map
    });
    let constructor = page_inventory.get(name).unwrap();
    constructor(params)
}

struct PageEntry {
    name: &'static str,
    constructor: WebPageConstructor,
}

pub struct PageProvider {
    // The "String identifier" from the URL mapper
    pub form_type: String,
    // The raw data to fill the struct, taken to construct the page.
    pub params: Option<PageParameters>,
}

impl PageProvider {
    pub fn new(form_type: &str, params: Option<PageParameters>) -> Self {
        Self {
            form_type: form_type.to_string(),
            params,
        }
    }

    pub fn get_instance(&mut self) -> Box<dyn WebPage> {
        construct_page(&self.form_type, self.params.take())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use wicket_macro::page_factory_config;

    use crate::components::ComponentId;
    use crate::request::Response;
    use crate::request::WebPage;

    struct TestPage {}

    impl WebPage for TestPage {
        fn render_component(
            &self,
            _id: ComponentId,
            response: &mut Response,
        ) -> std::io::Result<()> {
            response.write_str("Render TestPage components")
        }
    }

    inventory::submit! {
        PageEntry {
            name: "test",
            constructor: |_params| {
                Box::new(TestPage{})
            }
        }
    }

    #[test]
    pub fn webpage_constructor_test() {
        let web_page = construct_page("test", None);
        let mut response = Response {
            body: Vec::new(),
            content_type: None,
            headers: None,
            status: 0,
        };
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(response.body, "Render TestPage components".as_bytes());
    }

    #[page_factory_config]
    struct ParameterizedPage {
        data: String,
    }

    impl WebPage for ParameterizedPage {
        fn render_component(
            &self,
            _id: ComponentId,
            response: &mut Response,
        ) -> std::io::Result<()> {
            response.write_str(format!("ParameterizedPage data : {}", &self.data).as_str())
        }
    }

    impl ParameterizedPage {
        fn create_from_params(page_parameters: Option<PageParameters>) -> Self {
            let data = page_parameters
                .as_ref()
                .and_then(|p| p.get("data"))
                .map(|np| np.value.clone())
                .unwrap_or_else(|| panic!("Parameter does not exist??"));

            Self { data }
        }
    }

    #[test]
    pub fn webpage_parameter_test() {
        let param = PageParameters::new().add("data".to_string(), "abc123".to_string());
        let web_page = construct_page("ParameterizedPage", Some(param));
        let mut response = Response {
            body: Vec::new(),
            content_type: None,
            headers: None,
            status: 0,
        };
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(response.body, "ParameterizedPage data : abc123".as_bytes());
    }
}
