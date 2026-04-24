use std::{io::Result, sync::Arc};

use crate::{
    ajax::AjaxContext,
    protocol::http::WebApplication,
    request::{Request, RequestHandler, RequestMapperLogic, Response},
};

pub struct RequestCycle {
    pub request: Request,
    pub response: Response,
    pub app: Arc<WebApplication>,
    ///Active handler
    pub handler: Option<Box<dyn RequestHandler>>,
    // The scheduled destination
    pub redirect_url: Option<String>,
    pub ajax_context: Option<AjaxContext>,
}

impl RequestCycle {
    pub fn new(
        app: Arc<WebApplication>,
        request: Request,
        response: Response,
        handler: Option<Box<dyn RequestHandler>>,
    ) -> Self {
        Self {
            app,
            request,
            response,
            handler,
            redirect_url: None,
            ajax_context: None,
        }
    }

    pub(crate) async fn process_request(&mut self) -> Result<()> {
        let handler = self
            .resolve_request_handler()
            .expect("Error: no handler found!");
        handler.respond(self)
    }

    pub(crate) fn to_response(&self) -> Response {
        todo!()
    }

    pub fn resolve_request_handler(&mut self) -> Option<Box<dyn RequestHandler>> {
        // We ask the application (via its SystemMapper) to find the handler
        let mapper = self
            .app
            .root_request_mapper
            .read()
            .expect("Could not access RwLock<WebApplication> ?");
        mapper.map_request(&self.request)
    }
}
