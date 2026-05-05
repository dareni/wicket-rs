use std::{
    collections::{hash_map::Entry, HashMap},
    sync::OnceLock,
};

use wicket_request::request::mapper::parameter::PageParameters;

use crate::{
    components::{PageType, WebPage},
    request::{cycle::RedirectAction, RequestHandler},
};

inventory::collect!(PageEntry);

type WebPageConstructor = fn(params: Option<PageParameters>) -> Box<dyn WebPage>;
static PAGE_FACTORY: OnceLock<HashMap<u32, &PageEntry>> = OnceLock::new();

fn create_page_factory_map() -> HashMap<u32, &'static PageEntry> {
    let mut page_map: HashMap<u32, &PageEntry> = HashMap::new();
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

pub fn construct_page(id: &PageType, params: Option<PageParameters>) -> Box<dyn WebPage> {
    let page_inventory = PAGE_FACTORY.get_or_init(&create_page_factory_map);

    let constructor = page_inventory
        .get(&id.id)
        .unwrap_or_else(|| {
            panic!(
                "Error: PageFactory does not contain name:'{}' id:'{}'",
                id.name, id.id
            )
        })
        .constructor;
    constructor(params)
}

struct PageEntry {
    id: &'static PageType,
    constructor: WebPageConstructor,
}

/// Fresh creation: page_type, params.
/// Identity: page_id, render_id.
pub struct PageProvider {
    pub page_type: &'static PageType,
    // The data taken to construct the page.
    pub params: Option<PageParameters>,
    // The instance of a page, caters to multiple tabs.
    pub page_id: u16,
    // State change snapshot within an instance.
    pub render_id: u16,
}

impl PageProvider {
    pub fn new(page_type: &'static PageType, params: Option<PageParameters>) -> Self {
        Self {
            page_type,
            params,
            page_id: 0,
            render_id: 0,
        }
    }

    pub fn get_instance(&mut self) -> Box<dyn WebPage> {
        construct_page(self.page_type, self.params.take())
    }
}

pub struct RedirectHandler {
    pub redirect_action: RedirectAction,
}
impl From<RedirectAction> for RedirectHandler {
    fn from(redirect_action: RedirectAction) -> Self {
        Self { redirect_action }
    }
}
impl RedirectHandler {}
impl RequestHandler for RedirectHandler {
    fn respond(
        &self,
        _cycle: &mut super::cycle::RequestCycle,
    ) -> std::io::Result<super::cycle::HandlerResult> {
        todo!()
    }

    fn get_response_page(&self) -> &Option<Box<dyn WebPage>> {
        todo!()
    }

    fn as_page_provider(&self) -> &Option<PageProvider> {
        todo!()
    }
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

    struct TestPage {}

    impl WebPage for TestPage {
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
        let web_page = construct_page(&TESTPAGE_ID, None);
        let mut response = Response {
            body: ResponseBody::Buffered(Vec::new()),
            content_type: None,
            headers: None,
            status: 0,
        };
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(
            match response.body {
                ResponseBody::Buffered(buf) => buf,
                _unreachable => vec![],
            },
            "Render TestPage components".as_bytes()
        );
    }

    #[wicket_page]
    struct ParameterizedPage {
        data: String,
    }

    impl WebPage for ParameterizedPage {
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
        let web_page = construct_page(&WICKETPAGEID_PARAMETERIZEDPAGE, Some(param));
        let mut response = Response {
            body: ResponseBody::Buffered(Vec::new()),
            content_type: None,
            headers: None,
            status: 0,
        };
        web_page
            .render_component(ComponentId::TagId(0), &mut response)
            .ok();
        assert_eq!(
            match response.body {
                ResponseBody::Buffered(buf) => buf,
                _unreachable => vec![],
            },
            "ParameterizedPage data : abc123".as_bytes()
        );
    }
}
