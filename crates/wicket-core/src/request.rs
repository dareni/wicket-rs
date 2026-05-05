pub mod cycle;
pub mod handler;
pub mod mapper;

use std::collections::HashMap;
use std::io::Error;
use std::io::Write;

use bytes::Bytes;
use http::request::Parts;
use url::Url;

use crate::components::WebPage;
use crate::request::cycle::HandlerResult;
use crate::request::cycle::RequestCycle;
use crate::request::handler::PageProvider;
use crate::request::mapper::{BookmarkableMapper, MountedMapper, PackageMapper, ResourceMapper};

pub enum RequestBody {
    None,
    Bytes(Bytes),
}

pub struct Request {
    pub parts: Parts,
    pub body: RequestBody,
}

impl Request {
    pub fn new(parts: Parts, body: RequestBody) -> Self {
        Self { parts, body }
    }
}

#[derive(Default)]
/// The handler sets the body type.
pub enum ResponseBody {
    /// Small, buffered responses (HTML, JSON)
    Buffered(Vec<u8>), //Vec::with_capacity(32 * 1024)
    /// Large, streaming responses (Files, dynamic video)
    Streaming(Box<dyn std::io::Read + Send>),
    #[default]
    /// For 302s or 204 No Content
    Empty,
}

impl ResponseBody {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        match self {
            ResponseBody::Buffered(buff) => buff.write(buf),
            _ => unreachable!(),
        }
    }
}

#[derive(Default)]
pub struct Response {
    body: ResponseBody,
    content_type: Option<String>,
    headers: Option<HashMap<String, String>>,
    /// Status code (e.g., 200)
    pub status: u16,
}

impl Response {
    pub fn new() -> Self {
        Self {
            body: ResponseBody::Empty,
            content_type: None,
            headers: None,
            status: 200,
        }
    }

    pub fn set_body(&mut self, body: ResponseBody) {
        self.body = body;
    }

    pub fn get_body(&self) -> &ResponseBody {
        &self.body
    }

    pub fn set_content_type(&mut self, content_type: impl Into<String>) {
        self.content_type = Some(content_type.into());
    }

    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let header_map = self.headers.get_or_insert(HashMap::with_capacity(2));
        header_map.insert(name.into(), value.into());
    }

    pub fn write_str(&mut self, buf: &str) -> std::result::Result<(), Error> {
        self.write_all(buf.as_bytes())
    }
}

impl Write for Response {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.body.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub enum RequestMapper {
    Mounted(MountedMapper),
    Package(PackageMapper),
    Resource(ResourceMapper),
    Bookmarkable(BookmarkableMapper),
    Custom(Box<dyn RequestMapperLogic>),
}

pub struct RequestMappingResult {
    pub handler: Box<dyn RequestHandler>,
    pub compatibility_score: i32, // Helps the App decide between two similar routes
}

/// The compatibility_score is contained in the return from map_request.
pub trait RequestMapperLogic: Send + Sync {
    fn map_request(&self, request: &Request) -> Option<RequestMappingResult>;

    /// Map the handler to a url eg for a link component. The link component creates a
    /// RenderPageRequestHandler containing the target class and request parameters via the
    /// PageProvider. Rendering triggers a map_url_for(handler) on the RequestCycle. RequestCycle
    /// hits each mapper until a url is generated.  The url is added to the href of the link.
    fn map_handler(&self, handler: &dyn RequestHandler) -> Option<Url>;
}

impl RequestMapperLogic for RequestMapper {
    fn map_request(&self, request: &Request) -> Option<RequestMappingResult> {
        match self {
            RequestMapper::Mounted(rm) => rm.map_request(request),
            RequestMapper::Package(rm) => rm.map_request(request),
            RequestMapper::Resource(rm) => rm.map_request(request),
            RequestMapper::Bookmarkable(rm) => rm.map_request(request),
            RequestMapper::Custom(rm) => rm.map_request(request),
        }
    }

    fn map_handler(&self, _handler: &dyn RequestHandler) -> Option<Url> {
        todo!()
    }
}

pub trait RequestHandler {
    fn respond(&self, cycle: &mut RequestCycle) -> std::io::Result<HandlerResult>;
    fn get_response_page(&self) -> &Option<Box<dyn WebPage>>;
    fn as_page_provider(&self) -> &Option<PageProvider>;
}
