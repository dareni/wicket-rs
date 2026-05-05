use std::collections::HashSet;

use crate::components::{ComponentId, InternalId, WebPage};
use crate::request::cycle::{HandlerResult, RedirectAction, RequestCycle};
use crate::request::handler::RedirectHandler;
use crate::request::{RequestHandler, Response};

#[derive(Default)]
pub struct AjaxContext {
    pub dirty_components: HashSet<InternalId>,
    pub prepend_js: Vec<String>,
    pub append_js: Vec<String>,
}

pub struct AjaxRequestTarget<'a> {
    // Source is RequestCycle
    context: &'a mut AjaxContext,
}

impl<'a> AjaxRequestTarget<'a> {
    pub fn add(&mut self, component_id: InternalId) {
        self.context.dirty_components.insert(component_id);
    }

    pub fn new(context: &'a mut AjaxContext) -> Self {
        AjaxRequestTarget { context }
    }

    pub fn append_javascript(&mut self, script: String) {
        self.context.append_js.push(script);
    }
}

pub fn test(response: &mut Response) -> std::io::Result<HandlerResult> {
    response
        .write_str("<ajax-response>")
        .map(|_| HandlerResult::Complete)
}

impl<'a> RequestHandler for AjaxRequestTarget<'a> {
    fn respond(&self, cycle: &mut RequestCycle) -> std::io::Result<HandlerResult> {
        let some_page = self.get_response_page();

        let RequestCycle { response, .. } = cycle;
        response.set_content_type("text/xml");
        response.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        response.write_str("<ajax-response>")?;
        if let Some(page) = some_page {
            for id in &self.context.dirty_components {
                response.write_str(&format!("<component id=\"{}\"><![CDATA[", id))?;
                let action = page.render_component(ComponentId::Internal(*id), response)?;
                if !matches!(action, RedirectAction::None) {
                    return Ok(HandlerResult::Schedule(Box::from(RedirectHandler::from(
                        action,
                    ))));
                }
                response.write_str("]]></component>")?;
            }
        }
        response.write_str("</ajax-response>")?;
        Ok(HandlerResult::Complete)
    }

    fn get_response_page(&self) -> &Option<Box<dyn WebPage>> {
        todo!()
    }

    fn as_page_provider(&self) -> &Option<crate::request::handler::PageProvider> {
        &None
    }
}
