use wicket_request::request::mapper::parameter::PageParameters;

use crate::{
    components::{PageType, WebPage},
    request::{
        cycle::{RedirectAction, SessionProvider},
        RequestHandler,
    },
    session::page_factory::construct_page,
};

/// Fresh creation: page_type, params.
/// Identity: page_id, render_id.
pub struct PageProvider {
    pub page_type: &'static PageType,
    // The data taken to construct the page.
    pub params: Option<PageParameters>,
    // The instance of a page, caters to multiple tabs.
    pub page_id: Option<u16>,
    // State change snapshot within an instance.
    pub render_id: Option<u16>,
}

impl PageProvider {
    pub fn new(page_type: &'static PageType, params: Option<PageParameters>) -> Self {
        Self {
            page_type,
            params,
            page_id: None,
            render_id: None,
        }
    }

    pub fn get_instance(&mut self) -> Box<dyn WebPage> {
        construct_page(self.page_type, self.params.take())
    }
    pub fn needs_session_lookup(&self) -> bool {
        self.page_id.is_some()
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
        _session_provider: &mut SessionProvider,
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
