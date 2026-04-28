use crate::request::{RequestMapper, RequestMapperLogic};

/// Replace java SystemMapper.
pub fn get_default_mappers() -> Vec<RequestMapper> {
    vec![RequestMapper::Mounted(MountedMapper::default())]
}

#[derive(Default)]
pub struct MountedMapper {}
impl RequestMapperLogic for MountedMapper {
    fn map_request(&self, _request: &super::Request) -> Option<super::RequestMappingResult> {
        todo!()
    }

    fn map_handler(&self, _handler: &dyn super::RequestHandler) -> Option<url::Url> {
        todo!()
    }
}
pub struct PackageMapper {}
impl RequestMapperLogic for PackageMapper {
    fn map_request(&self, _request: &super::Request) -> Option<super::RequestMappingResult> {
        todo!()
    }

    fn map_handler(&self, _handler: &dyn super::RequestHandler) -> Option<url::Url> {
        todo!()
    }
}
pub struct ResourceMapper {}
impl RequestMapperLogic for ResourceMapper {
    fn map_request(&self, _request: &super::Request) -> Option<super::RequestMappingResult> {
        todo!()
    }

    fn map_handler(&self, _handler: &dyn super::RequestHandler) -> Option<url::Url> {
        todo!()
    }
}
pub struct BookmarkableMapper {}
impl RequestMapperLogic for BookmarkableMapper {
    fn map_request(&self, _request: &super::Request) -> Option<super::RequestMappingResult> {
        todo!()
    }

    fn map_handler(&self, _handler: &dyn super::RequestHandler) -> Option<url::Url> {
        todo!()
    }
}
