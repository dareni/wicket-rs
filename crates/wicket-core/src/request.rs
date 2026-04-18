pub mod cycle;
pub mod mapper;

use core::str;

use bytes::Bytes;
use http::request::Parts;
use url::Url;

use crate::request::cycle::RequestCycle;
use crate::request::mapper::{
    BookmarkableMapper, MountedMapper, PackageMapper, ResourceMapper, SystemMapper,
};

pub enum Body {
    None,
    Bytes(Bytes),
}

pub struct Request {
    pub parts: Parts,
    pub body: Body,
}

impl Request {
    pub fn new(parts: Parts, body: Body) -> Self {
        Self { parts, body }
    }
}

#[derive(Default)]
pub struct Response {}

impl Response {
    pub fn new() -> Self {
        Response::default()
    }

    pub fn write(&self, _sequence: &str) {
        todo!()
    }

    pub(crate) fn set_content_type(&self, _type: &str) {
        todo!()
    }
}

pub enum RequestMapper {
    Mounted(MountedMapper),
    Package(PackageMapper),
    Resource(ResourceMapper),
    Bookmarkable(BookmarkableMapper),
    System(SystemMapper),
    Custom(Box<dyn RequestMapperLogic>),
}

impl RequestMapperLogic for RequestMapper {
    fn map_request(&self, _request: &Request) -> Option<Box<dyn RequestHandler>> {
        todo!()
    }

    fn map_handler(&self, _handler: &dyn RequestHandler) -> Option<Url> {
        todo!()
    }

    fn get_compatibility_score(&self, _request: &Request) -> i32 {
        todo!()
    }
}

pub trait RequestMapperLogic: Send + Sync {
    fn map_request(&self, request: &Request) -> Option<Box<dyn RequestHandler>>;
    fn map_handler(&self, handler: &dyn RequestHandler) -> Option<Url>;
    fn get_compatibility_score(&self, request: &Request) -> i32;
}

pub trait RequestHandler {
    fn respond(&self, cycle: &mut RequestCycle);
}

pub trait RequestLogic {
    // The "Do the work" method
    fn respond(&self, cycle: &mut RequestCycle);
}

pub struct CompoundRequestMapper {
    // We use a Vec of Trait Objects
    mappers: Vec<RequestMapper>,
}

impl CompoundRequestMapper {
    pub fn new(mappers: Vec<RequestMapper>) -> Self {
        Self { mappers }
    }

    pub fn add(&mut self, mapper: RequestMapper) {
        self.mappers.push(mapper)
    }

    pub fn get_mappers(&self) -> &Vec<RequestMapper> {
        &self.mappers
    }
}

impl RequestMapperLogic for CompoundRequestMapper {
    fn map_request(&self, request: &Request) -> Option<Box<dyn RequestHandler>> {
        self.mappers
            .iter()
            // 1. Get the score and the mapper together
            .map(|mapper| (mapper.get_compatibility_score(request), mapper))
            // 2. Only consider mappers that actually claim they can handle it (score > 0)
            .filter(|(score, _)| *score > 0)
            // 3. Find the one with the highest score
            // In case of ties, the one added most recently (first in Vec) usually wins
            .max_by_key(|(score, _)| *score)
            // 4. Use the winning mapper to generate the handler
            .and_then(|(_, mapper)| mapper.map_request(request))
    }

    fn get_compatibility_score(&self, request: &Request) -> i32 {
        // A CompoundMapper's score is usually the highest score among its children
        self.mappers
            .iter()
            .map(|m| m.get_compatibility_score(request))
            .max()
            .unwrap_or(0)
    }

    fn map_handler(&self, handler: &dyn RequestHandler) -> Option<Url> {
        // Reverse mapping: finding a URL for a Page or Resource
        // Usually, the first mapper that provides a non-none URL wins
        self.mappers
            .iter()
            .find_map(|mapper| mapper.map_handler(handler))
    }
}
