use std::{io::Result, sync::Arc};

use url::Url;

use crate::{
    components::WebPage,
    protocol::http::WebApplication,
    request::{Request, RequestHandler, RequestMapperLogic, RequestMappingResult, Response},
};

pub enum RedirectAction {
    /// No navigation change.
    None,
    /// Post, redirect, get  (302->/get)
    RedirectSelf,
    /// Stop current flow, navigate to a new page
    Redirect(Box<dyn WebPage>),
    /// Stop current flow, go to a hardcoded URL (external)
    RedirectUrl(String),
}

pub enum HandlerResult {
    Complete,
    Schedule(Box<dyn RequestHandler>),
}

pub struct RequestCycle {
    pub request: Request,
    pub response: Response,
    pub app: Arc<WebApplication>,
}

impl RequestCycle {
    pub fn new(app: Arc<WebApplication>, request: Request, response: Response) -> Self {
        Self {
            app,
            request,
            response,
        }
    }

    pub(crate) async fn process_request(&mut self) -> Result<()> {
        let mut handler = self
            .resolve_request_handler(&self.request)
            .expect("Error: no handler found!")
            .handler;

        loop {
            match handler.respond(self)? {
                HandlerResult::Complete => break,
                HandlerResult::Schedule(next_handler) => handler = next_handler,
            };
        }
        Ok(())
    }

    pub(crate) fn to_response(&self) -> Response {
        todo!()
    }

    /// For each mapper, construct a handler to derive a compatibility_score.
    /// Return the result with the maximum score.
    fn resolve_request_handler(&self, request: &Request) -> Option<RequestMappingResult> {
        self.app
            .app_request_mappers
            .read()
            .unwrap_or_else(|e| {
                panic!(
                    "Error accessing app_request_mappers for handler resolution! {}",
                    e
                )
            })
            .iter()
            // Generate a RequestMappingResult for each mapper.
            .filter_map(|mapper| mapper.map_request(request))
            // Only consider mapper able to handle the request.
            .filter(|rmr| rmr.compatibility_score > 0)
            // In case of ties, the mapper added most recently wins.
            .max_by_key(|rmr| rmr.compatibility_score)
    }

    // Reverse mapping: finding a URL for a Page or Resource
    // Usually, the first mapper that provides a non-none URL wins
    pub fn map_url_for(&self, handler: &dyn RequestHandler) -> Option<Url> {
        self.app
            .app_request_mappers
            .read()
            .unwrap_or_else(|e| {
                panic!(
                    "Error accessing app_request_mappers for reverse mapping! {}",
                    e
                )
            })
            .iter()
            .find_map(|mapper| mapper.map_handler(handler))
    }
}
