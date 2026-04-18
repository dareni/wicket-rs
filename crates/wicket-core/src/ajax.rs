use std::collections::HashSet;

use crate::components::InternalId;
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
    fn respond(&self, cycle: &mut crate::request::cycle::RequestCycle) {
        cycle.response.set_content_type("text/xml");
        cycle
            .response
            .write("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        cycle.response.write("<ajax-response>");
        if let Some(url) = &cycle.redirect_url {
            cycle
                .response
                .write(&format!("<redirect><![CDATA[{}]]></redirect>", url));
            return;
        } else {
            let page_opt = cycle.get_response_page().as_ref();
            if let Some(page) = page_opt {
                // Render the specific components registered in the target
                if let Some(ajax_context) = &cycle.ajax_context {
                    for id in &ajax_context.dirty_components {
                        cycle
                            .response
                            .write(&format!("<component id=\"{}\"><![CDATA[", id));
                        // Call back into the page/component to render its markup
                        page.render_component(*id, &cycle.response);
                        cycle.response.write("]]></component>");
                    }
                }
            }
        }
        cycle.response.write("</ajax-response>");
    }
}
