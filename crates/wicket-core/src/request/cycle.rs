use std::{io::Result, sync::Arc};

use tokio::sync::{Mutex, MutexGuard};
use url::Url;

use crate::{
    components::WebPage,
    protocol::http::WebApplication,
    request::{Request, RequestHandler, RequestMapperLogic, RequestMappingResult, Response},
    session::SessionData,
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

/// An anchor outside of the RequestCycle.
#[derive(Default)]
pub struct SessionProvider {
    pub session_handle: Option<Arc<Mutex<SessionData>>>,
}

pub struct RequestCycle<'a> {
    pub request: Request,
    pub response: Response,
    pub app: Arc<WebApplication>,
    session_guard: Option<MutexGuard<'a, SessionData>>,
}

impl<'a> RequestCycle<'a> {
    pub fn new(app: Arc<WebApplication>, request: Request, response: Response) -> Self {
        Self {
            app,
            request,
            response,
            session_guard: None,
        }
    }

    pub(crate) async fn process_request(
        &mut self,
        session_provider: &mut SessionProvider,
    ) -> Result<()> {
        let mut handler = self
            .resolve_request_handler(&self.request)
            .expect("Error: no handler found!")
            .handler;

        loop {
            match handler.respond(self, session_provider)? {
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

    /// Reverse mapping: finding a URL for a Page or Resource.
    /// Usually, the first mapper that provides a non-none URL wins
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

    /// Store the mutex in the provider and the mutex guard in the request cycle.
    pub async fn get_session_mut(
        &mut self,
        session_provider: &'a mut SessionProvider,
    ) -> Option<&mut SessionData> {
        if self.session_guard.is_none() {
            // Initial lock: Fetch from DashMap and Lock Mutex
            let session_id = self.request.get_session_id()?;
            let handle = self
                .app
                .get_session_registry()
                .get_session_handle(session_id)?;

            session_provider.session_handle = Some(handle);
            self.session_guard = Some(
                session_provider
                    .session_handle
                    .as_ref()
                    .unwrap()
                    .lock()
                    .await,
            );
        }
        self.session_guard.as_deref_mut()
    }
}
