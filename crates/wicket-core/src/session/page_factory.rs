use std::{
    collections::{hash_map::Entry, HashMap},
    sync::OnceLock,
};

use wicket_request::request::mapper::parameter::PageParameters;

use crate::components::{PageType, WebPage};

inventory::collect!(PageEntry);

type WebPageConstructor = fn(params: Option<PageParameters>) -> Box<dyn WebPage>;
static PAGE_FACTORY: OnceLock<HashMap<u16, &PageEntry>> = OnceLock::new();

fn create_page_factory_map() -> HashMap<u16, &'static PageEntry> {
    let mut page_map: HashMap<u16, &PageEntry> = HashMap::new();
    for entry in inventory::iter::<PageEntry> {
        match page_map.entry(entry.id.id) {
            Entry::Vacant(e) => {
                e.insert(entry);
            }
            Entry::Occupied(_) => {
                let collider = page_map.get(&entry.id.id).unwrap();
                panic!(
                    "Error: Page name hash collision for names:{} and {}",
                    entry.id.name, collider.id.name
                );
            }
        }
    }
    page_map
}
pub fn construct_page_type(
    id: &PageType,
    params: Option<PageParameters>,
) -> Option<Box<dyn WebPage>> {
    construct_page(id.id, params)
}

pub fn construct_page(id: u16, params: Option<PageParameters>) -> Option<Box<dyn WebPage>> {
    let page_inventory = PAGE_FACTORY.get_or_init(&create_page_factory_map);
    page_inventory.get(&id).map(|pe| (pe.constructor)(params))
}

struct PageEntry {
    id: &'static PageType,
    constructor: WebPageConstructor,
}

#[cfg(test)]
mod test {
    use wicket_macro::wicket_page;
    use wicket_macro_support::hash_string;
    use wicket_request::request::mapper::parameter::PageParameters;
    use wicket_util::constants::file_ext;

    use crate::components::ComponentId;
    use crate::components::PageIdentifier;
    use crate::markup::loader::MarkupResourceLocationUtil;
    use crate::request::cycle::RedirectAction;
    use crate::request::Response;
    use crate::request::ResponseBody;

    use super::*;

    #[derive(Clone)]
    struct TestPage {}

    impl WebPage for TestPage {
        // Use render_component() to test the page instant.
        fn render_component(
            &self,
            _id: ComponentId,
            response: &mut Response,
        ) -> std::io::Result<RedirectAction> {
            response.write_str("Render TestPage components")?;
            Ok(RedirectAction::None)
        }
    }

    static TESTPAGE_ID: PageType = PageType {
        id: hash_string("TestPage"),
        name: "TestPage",
    };

    impl PageIdentifier for TestPage {
        fn get_page_identity(&self) -> &PageType {
            &TESTPAGE_ID
        }
    }

    inventory::submit! {
        PageEntry {
            id: &TESTPAGE_ID,
            constructor: |_params| {
                Box::new(TestPage{})
            }
        }
    }

    #[test]
    pub fn webpage_constructor_test() {
        let web_page = construct_page_type(&TESTPAGE_ID, None).unwrap();
        let mut response = Response::new();
        response.set_body(ResponseBody::Buffered(vec![]));
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(
            match response.get_body() {
                ResponseBody::Buffered(buf) => buf,
                _unreachable => panic!("Not a buffered response???"),
            },
            "Render TestPage components".as_bytes()
        );
    }

    #[wicket_page]
    struct ParameterizedPage {
        data: String,
    }

    impl WebPage for ParameterizedPage {
        // Use render_component() to test the page instant.
        fn render_component(
            &self,
            _id: ComponentId,
            response: &mut Response,
        ) -> std::io::Result<RedirectAction> {
            response.write_str(format!("ParameterizedPage data : {}", &self.data).as_str())?;
            Ok(RedirectAction::None)
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
        let web_page = construct_page_type(&WICKETPAGEID_PARAMETERIZEDPAGE, Some(param)).unwrap();
        let mut response = Response::new();
        response.set_body(ResponseBody::Buffered(vec![]));
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(
            match response.get_body() {
                ResponseBody::Buffered(buf) => buf,
                _unreachable => panic!("Not a buffered response???"),
            },
            "ParameterizedPage data : abc123".as_bytes()
        );
    }
}
