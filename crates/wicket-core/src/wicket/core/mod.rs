use crate::components::InternalId;

pub mod behavior;
pub mod central;
pub mod markup;
pub mod protocol;
pub mod settings;

pub trait Component {
    fn markup_id(&self) -> &str;
    fn set_internal_id(&self, id: InternalId);
}
pub struct MarkupContainer {}

#[derive(Default)]
pub struct MarkupIdGenerator {}

#[derive(Default)]
pub struct SystemMapper {}

impl SystemMapper {
    pub fn new() -> Self {
        Self::default()
    }
}
