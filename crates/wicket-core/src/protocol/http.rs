use std::io::Error;
use std::sync::Arc;
use std::sync::RwLock;

use crate::request::cycle::RequestCycle;
use crate::request::mapper::get_default_mappers;
use crate::request::{Request, RequestMapper, Response};
use crate::session::SessionRegistry;

pub struct WebApplication {
    pub app_request_mappers: RwLock<Vec<RequestMapper>>,
    pub sessions: Arc<SessionRegistry>,
}

impl Default for WebApplication {
    fn default() -> Self {
        Self {
            app_request_mappers: RwLock::from(get_default_mappers()),
            sessions: Arc::from(SessionRegistry::default()),
        }
    }
}
impl WebApplication {
    /// Add a request mapper.
    pub fn mount(&mut self, pos: usize, mapper: RequestMapper) {
        let mut map = self
            .app_request_mappers
            .write()
            .expect("Error locking root_request_mapper?");
        let idx = if pos > map.len() { map.len() } else { pos };
        map.insert(idx, mapper);
    }

    /// Port of WicketFilter.processRequest()
    /// This is the entry point from the hyper bridge.
    pub async fn process_request(self: &Arc<Self>, request: Request) -> Result<Response, Error> {
        // 1. Setup the RequestCycle
        let mut cycle = self.create_request_cycle(request);

        // 2. Execute the lifecycle (The "Heavy Lifting")
        // This mirrors RequestCycle.process() in Java
        cycle.process_request().await?;

        // 3. Finalize and return
        Ok(cycle.to_response())
    }

    pub fn create_request_cycle(self: &Arc<Self>, request: Request) -> RequestCycle {
        RequestCycle::new(self.clone(), request, Response::new())
    }

    pub fn get_session_registry(&self) -> Arc<SessionRegistry> {
        self.sessions.clone()
    }
}
