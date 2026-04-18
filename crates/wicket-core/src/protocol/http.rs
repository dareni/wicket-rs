use std::sync::Arc;
use std::sync::RwLock;

use crate::request::cycle::RequestCycle;
use crate::request::RequestHandler;
use crate::request::RequestMapperLogic;
use crate::request::{CompoundRequestMapper, Request, RequestMapper, Response};

use crate::request::mapper::SystemMapper;

pub struct WebApplication {
    pub root_request_mapper: RwLock<CompoundRequestMapper>,
}

impl WebApplication {
    pub fn new(user_mappers: Vec<RequestMapper>) -> Self {
        let mut mappers = user_mappers;

        // Add default Wicket mappers (System, Resource, etc.)
        mappers.push(RequestMapper::System(SystemMapper {}));

        Self {
            root_request_mapper: RwLock::from(CompoundRequestMapper::new(mappers)),
        }
    }

    pub fn resolve_handler(&self, request: &Request) -> Option<Box<dyn RequestHandler>> {
        let mapper = self.root_request_mapper.read().unwrap();
        mapper.map_request(request)
    }

    pub fn mount(&mut self, mapper: RequestMapper) {
        let mut map = self
            .root_request_mapper
            .write()
            .expect("Error locking root_request_mapper?");

        map.add(mapper);
    }

    /// Port of WicketFilter.processRequest()
    /// This is the entry point from the hyper bridge.
    pub async fn process_request(self: &Arc<Self>, request: Request) -> Response {
        // 1. Setup the RequestCycle
        let mut cycle = self.create_request_cycle(request);

        // 2. Execute the lifecycle (The "Heavy Lifting")
        // This mirrors RequestCycle.process() in Java
        cycle.process_request().await;

        // 3. Finalize and return
        cycle.to_response()
    }

    pub fn create_request_cycle(self: &Arc<Self>, request: Request) -> RequestCycle {
        RequestCycle::new(self.clone(), request, Response::new(), None)
    }
}
