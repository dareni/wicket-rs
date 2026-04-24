use std::collections::HashSet;

use crate::components::{ComponentId, InternalId, WebPage};
use crate::request::cycle::RequestCycle;
use crate::request::RequestHandler;

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

impl<'a> RequestHandler for AjaxRequestTarget<'a> {
    fn respond(&self, cycle: &mut RequestCycle) {
        let some_page = self.get_response_page();

        let RequestCycle {
            response,
            redirect_url,
            ajax_context,
            ..
        } = cycle;
        response.set_content_type("text/xml");
        response.write("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        response.write("<ajax-response>");
        if let Some(url) = redirect_url {
            response.write(&format!("<redirect><![CDATA[{}]]></redirect>", url));
            return;
        } else if let Some(page) = some_page {
            // Render the specific components registered in the target
            if let Some(ajax_context) = ajax_context {
                for id in &ajax_context.dirty_components {
                    response.write(&format!("<component id=\"{}\"><![CDATA[", id));
                    // Call back into the page/component to render its markup
                    page.render_component(ComponentId::Internal(*id), response);
                    response.write("]]></component>");
                }
            }
        }
        response.write("</ajax-response>");
    }

    fn get_response_page(&self) -> &Option<Box<dyn WebPage>> {
        todo!()
    }
}
