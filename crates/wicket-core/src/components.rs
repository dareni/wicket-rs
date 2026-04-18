use std::fmt::Display;

use crate::request::Response;

pub trait Component {
    fn markup_id(&self) -> &str;
    fn set_internal_id(&self, id: InternalId);
}
pub struct MarkupContainer {}

#[derive(Default)]
pub struct MarkupIdGenerator {}

pub trait Page {
    fn render_component(&self, _id: InternalId, _response: &Response) {
        todo!()
    }
}

pub struct WebPage {
    id_counter: u16,
    components: Vec<Box<dyn Component>>,
}

impl WebPage {
    fn next_id(&mut self) -> InternalId {
        let id = InternalId::from(self.id_counter);
        // Safety check for u16 overflow
        self.id_counter = self
            .id_counter
            .checked_add(1)
            .expect("InternalId overflow: Too many components on one page");
        id
    }

    pub fn add(&mut self, component: Box<dyn Component>) -> InternalId {
        let id = self.next_id();
        component.set_internal_id(id);
        self.components.insert(id.into(), component);
        id
    }
}

/// A type-safe wrapper for component IDs within a single Page.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternalId(pub(crate) u16);

impl From<u16> for InternalId {
    #[inline]
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<InternalId> for usize {
    #[inline]
    fn from(id: InternalId) -> Self {
        id.0 as usize
    }
}

impl Display for InternalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
