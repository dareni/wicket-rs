use std::{collections::HashMap, fmt::Display};

use crate::request::Response;

pub trait Component {
    fn markup_id(&self) -> &str;
    fn set_internal_id(&self, id: InternalId);
    fn render(&self, response: &Response);
}
pub struct MarkupContainer {}

#[derive(Default)]
pub struct MarkupIdGenerator {}

pub enum ComponentId {
    Internal(InternalId),
    TagId(u16),
}

pub trait WebPage {
    ///Render the component from ajax context
    fn render_component(&self, id: ComponentId, response: &mut Response);
}

pub struct Page {
    //Internal component id, should this be the index into components?.
    id_counter: u16,
    //Unique Id for this page instance.
    _instance_id: u8,
    //central page component store
    components: Vec<Box<dyn Component>>,
    tag_id_map: HashMap<u16, InternalId>,
    //direct children
    children: Vec<u16>,
}

impl Page {
    fn next_id(&mut self) -> InternalId {
        let id = InternalId::from(self.id_counter);
        // Safety check for u16 overflow
        self.id_counter = self
            .id_counter
            .checked_add(1)
            .expect("InternalId overflow: Too many components on one page");
        id
    }

    pub fn store(&mut self, component: Box<dyn Component>) -> InternalId {
        let id = self.next_id();
        component.set_internal_id(id);
        self.components.insert(id.into(), component);
        id
    }

    pub fn add(&mut self, component: Box<dyn Component>) {
        let id = self.store(component);
        self.children.push(id.into());
    }
}

impl WebPage for Page {
    fn render_component(&self, id: ComponentId, response: &mut Response) {
        let component_id: u16 = match id {
            ComponentId::Internal(internal) => internal.into(),
            ComponentId::TagId(id) => {
                let internal_id = self
                    .tag_id_map
                    .get(&id)
                    .unwrap_or_else(|| panic!("No mapping to InternalId for tagid:{}", id));
                (*internal_id).into()
            }
        };
        let component = self
            .components
            .get(component_id as usize)
            .unwrap_or_else(|| {
                panic!(
                    "Component id:{} does not exist in page cache.",
                    component_id
                )
            });
        component.render(response);
    }
}
/// A type-safe wrapper for component IDs within a single Page.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InternalId(pub(crate) u16);

impl From<InternalId> for u16 {
    fn from(value: InternalId) -> Self {
        value.0
    }
}

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
