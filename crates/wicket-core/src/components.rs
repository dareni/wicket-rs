use std::io::Write;
use std::{collections::HashMap, fmt::Display};

use dyn_clone::{clone_trait_object, DynClone};

use crate::request::cycle::RedirectAction;
use crate::request::Response;

pub trait Component: DynClone {
    fn markup_id(&self) -> &str;
    fn set_internal_id(&self, id: InternalId);
    fn get_internal_id(&self) -> Option<InternalId>;
    fn render(&self, response: &dyn Write) -> std::io::Result<RedirectAction>;
    fn get_parent(&self) -> Option<InternalId>;
    fn set_parent(&self, index: InternalId);
}
clone_trait_object!(Component);

/// A container of components with associated HTML/XML markup.
///
/// This trait requires a unique identifier (typically auto-generated
/// via proc-macros) which serves as the key to resolve the container's
/// markup stream.
///
/// ### Usage
/// **For Pages:** The unique identifier is used for dynamic runtime
///     construction of the component tree.
/// **For Standard Containers:** The identifier is used as a lookup key
///     to retrieve pre-parsed markup from the cache.
pub trait MarkupContainer: MarkupIdentifier {
    ///  Render the child component from create or ajax context.
    fn render_component(
        &self,
        id: ComponentId,
        response: &mut Response,
    ) -> std::io::Result<RedirectAction>;
}

#[derive(Default)]
pub struct MarkupIdGenerator {}

pub enum ComponentId {
    Internal(InternalId),
    TagId(u16),
}

/// A unique identifier for a MarkupContainer: page, panel, fragment.
pub struct MarkupType {
    pub id: u16,
    pub name: &'static str,
}

/// Implemented by proc_macro_derive wicket_page.
pub trait MarkupIdentifier {
    fn get_markup_identity(&self) -> &MarkupType;
}

// TODO: add Send + Sync for disk storage.
pub trait WebPage: MarkupContainer + DynClone {
    fn init(&self) {}
}
clone_trait_object!(WebPage);

#[derive(Clone)]
pub struct Page {
    // Unique Id for this page instance.
    _instance_id: u8,
    // Central page component store, contains all page components
    // including of the children of list views, parent pages(markup inheritance)
    // and their components,.
    // Each container component has a child list and access to page components.
    components: Vec<Box<dyn Component>>,
    // Indices of the pages: BasePage, SubBasePage.
    _inheritance_chain: Vec<usize>,
    tag_id_map: HashMap<u16, InternalId>,
    // Direct children of the page.
    children: Vec<u16>,
}

impl MarkupIdentifier for Page {
    fn get_markup_identity(&self) -> &MarkupType {
        panic!("Error: Missing proc_macro_derive. Add wicket_page attribute to the page.")
    }
}

impl Page {
    pub fn store(&mut self, component: Box<dyn Component>) -> InternalId {
        let id = InternalId::from(self.components.len());
        if component.get_internal_id().is_some() {
            panic!("Component {} is already registered!", component.markup_id());
        }
        component.set_internal_id(id);
        self.components.push(component);
        id
    }

    pub fn add(&mut self, component: Box<dyn Component>) {
        let id = self.store(component);
        self.children.push(id.into());
    }

    ///  Performed once per page type. The bind result is cache with the markup element vector.
    pub fn bind_markup() {}
}

impl MarkupContainer for Page {
    fn render_component(
        &self,
        id: ComponentId,
        response: &mut Response,
    ) -> std::io::Result<RedirectAction> {
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
        component.render(response)
    }
}
/// A type-safe wrapper for component IDs within a single Page.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
// pub struct InternalId(pub(crate) u16);
pub struct InternalId(u16);

impl From<usize> for InternalId {
    fn from(value: usize) -> Self {
        let val = u16::try_from(value)
            .unwrap_or_else(|_| panic!("Component Id exceeded maximum count {}.", value));
        Self(val)
    }
}

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

pub enum PageHandle<'a> {
    Borrowed {
        page: &'a dyn WebPage,
        dirty: bool,
    },
    Owned {
        page: Box<dyn WebPage + 'a>,
        dirty: bool,
    },
}

impl<'a> std::ops::Deref for PageHandle<'a> {
    type Target = dyn WebPage + 'a;

    fn deref(&self) -> &Self::Target {
        match self {
            PageHandle::Borrowed { page, dirty: _ } => *page,
            PageHandle::Owned { page, dirty: _ } => page.as_ref(),
        }
    }
}

impl<'a> PageHandle<'a> {
    pub fn as_trait(&self) -> &dyn WebPage {
        match self {
            PageHandle::Borrowed { page, dirty: _ } => *page,
            PageHandle::Owned { page, dirty: _ } => page.as_ref(),
        }
    }

    pub fn to_mut(&mut self) -> &mut (dyn WebPage + 'a) {
        if let PageHandle::Borrowed { page, dirty: _ } = *self {
            *self = PageHandle::Owned {
                page: dyn_clone::clone_box(page),
                dirty: false,
            };
        }

        match self {
            PageHandle::Owned { page, dirty: _ } => page.as_mut(),
            _ => unreachable!(),
        }
    }

    pub fn into_owned(self) -> Box<dyn WebPage + 'a> {
        match self {
            PageHandle::Borrowed { page, dirty: _ } => dyn_clone::clone_box(page),
            PageHandle::Owned { page, dirty: _ } => page,
        }
    }
}
