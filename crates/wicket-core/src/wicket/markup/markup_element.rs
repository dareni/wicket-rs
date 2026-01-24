use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Range;
use std::rc::Rc;
use std::slice::Iter;

use bitflags::bitflags;
use wicket_request::wicket::request::Response;

use crate::wicket::behavior::Behavior;
use crate::wicket::markup::parser::filter::HtmlHandler;
use crate::wicket::markup::parser::xml_tag::{TagType, XmlTag};
use crate::wicket::{Component, MarkupContainer};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ComponentTagFlags: u8 {
        const NONE               = 0b00000000;
        const AUTOLINK           = 0b00000001;
        const MODIFIED           = 0b00000010;
        const IGNORE             = 0b00000100;
        const AUTO_COMPONENT     = 0b00001000;
        const NO_CLOSE_TAG       = 0b00010000;
        const RENDER_RAW         = 0b00100000;
        const CONTAINS_WICKET_ID = 0b01000000;
    }
}

pub trait AutoComponentFactory {
    fn new_component(container: &MarkupContainer, tag: &ComponentTag);
}

pub enum MarkupElement {
    /// Plain HTML, text, or whitespace that doesn't interact with Wicket logic.
    RawMarkup(RawMarkup),

    /// A Wicket-aware tag (e.g., <span wicket:id="label">) or a tag modified by a wicket filter.
    /// This carries the promoted ComponentTag data.
    ComponentTag(ComponentTag),

    /// Comments, CDATA, or specialized fragments like the
    /// Downlevel-Revealed Conditional Comments.
    SpecialTag(SpecialTag),
}

pub struct RawMarkup {
    pub text_range: Range<usize>,
}

pub struct SpecialTag {
    pub tag: XmlTag,
}

/// A subclass of MarkupElement which represents a "significant" markup tag, such as a component open
/// tag. Insignificant markup tags (those which are merely concerned with markup formatting
/// operations and do not denote components or component nesting) are coalesced into instances of
/// RawMarkup (also a subclass of MarkupElement).
///
pub struct ComponentTag {
    //  If close tag, than reference to the corresponding open tag.
    pub open_tag: Option<Box<ComponentTag>>,

    // The underlying xml tag.
    pub tag: XmlTag,

    /// Boolean flags.
    pub flags: ComponentTagFlags,

    /// By default this is equal to the wicket:id="xxx" attribute value, but may be provided e.g. for
    /// auto-tags
    pub id: String,

    /// In case of inherited markup, the base and the extended markups are merged and the information
    /// about the tags origin is lost. In some cases like wicket:head and wicket:link this
    /// information however is required.
    pub markup_ref: Option<Rc<dyn Component>>,

    /// Added behaviours.
    pub behaviors: Option<Vec<Box<dyn Behavior>>>,

    /// Filters and Handlers may add their own attributes to the tag.
    pub user_data: Option<HashMap<String, String>>,
}

impl Default for ComponentTag {
    fn default() -> Self {
        Self {
            open_tag: Option::None,
            tag: XmlTag::new(),
            flags: ComponentTagFlags::NONE,
            id: "".to_owned(),
            markup_ref: Option::None,
            behaviors: Option::None,
            user_data: Option::None,
        }
    }
}

impl ComponentTag {
    pub fn new(name_range: Range<usize>, tag_type: &TagType) -> Self {
        let tag = XmlTag::new_from(name_range, tag_type);
        ComponentTag {
            tag,
            ..Default::default()
        }
    }

    pub fn from_xml_tag(xml_tag: XmlTag) -> Self {
        ComponentTag {
            tag: xml_tag,
            ..Default::default()
        }
    }

    pub fn add_behavior<T: Behavior + 'static>(&mut self, behavior: T) {
        match &mut self.behaviors {
            Some(vec) => vec.push(Box::new(behavior)),
            None => {
                self.behaviors = Some(vec![Box::new(behavior)]);
            }
        }
    }

    pub fn has_behaviors(&self) -> bool {
        match &self.behaviors {
            Some(vec) => !vec.is_empty(),
            None => false,
        }
    }

    pub fn get_behaviors<'a>(&'a self) -> std::slice::Iter<'a, Box<dyn Behavior>> {
        match &self.behaviors {
            Some(vec) => vec.iter(),
            None => [].iter() as Iter<Box<dyn Behavior>>,
        }
    }

    //  Returns true when this tag close the given open tag.
    pub fn closes(&self, open: &ComponentTag, components: &[MarkupElement]) -> bool {
        match self.open_tag.as_deref() {
            Some(ct) => {
                if std::ptr::eq(ct, open) {
                    true
                } else {
                    self.get_xml_tag().closes(open.get_xml_tag(), components)
                }
            }
            None => false,
        }
    }

    /// If autolink is set to true, href attributes will automatically be converted into Wicket
    /// bookmarkable URLs.
    pub fn enable_autolink(&mut self, autolink: bool) {
        if autolink {
            self.flags.insert(ComponentTagFlags::AUTOLINK);
        } else {
            self.flags.remove(ComponentTagFlags::AUTOLINK);
        }
    }

    /// True if autolink is enabled and the tag contains a href attrib.
    pub fn is_autolink_enabled(&self) -> bool {
        (self.flags & ComponentTagFlags::AUTOLINK).is_empty()
    }

    /// Get the component id.
    pub fn get_id(&self) -> &String {
        &self.id
    }

    /// Get the open tag.
    pub fn get_open_tag(&self) -> &Option<Box<ComponentTag>> {
        &self.open_tag
    }

    /// Return true when this tag does not require a closing tag.
    pub fn requires_close_tag(&self) -> bool {
        if self.get_xml_tag().namespace().is_none() {
            HtmlHandler::requires_close_tag(self.get_xml_tag().name().as_ref())
        } else {
            let ns = self.get_xml_tag().namespace().unwrap();
            let q_name = format!("{}:{}", ns, self.get_xml_tag().name());

            HtmlHandler::requires_close_tag(&q_name)
        }
    }

    /// St the id of the component. The value is usuall taken from the tag id attribute e.g.
    /// wicket:id="componentid".
    pub fn set_id(&mut self, id: &str) {
        self.id = id.into();
    }

    pub fn write_synthetic_close_tag(&self, response: &Response) {
        response.write("</");
        if let Some(ns) = self.get_xml_tag().namespace() {
            response.write(ns.as_ref());
            response.write(":");
        }
        response.write(self.get_xml_tag().name().as_ref());
        response.write(">");
    }

    /// Write tag to response.
    /// When strip_wicket_attributes is true, wicket:id is removed from the output.
    /// The default namespace is 'wicket'.
    pub fn write_output(
        &self,
        response: &Response,
        strip_wicket_attributes: bool,
        namespace: &str,
    ) {
        response.write("<");
        if self.get_xml_tag().tag_type().eq(&TagType::Close) {
            response.write("/");
        }
        if let Some(ns) = self.get_xml_tag().namespace() {
            response.write(ns.as_ref());
            response.write(":");
        }
        response.write(self.get_xml_tag().name().as_ref());
        let mut namespace_prefix: Option<Rc<String>> = None;
        if strip_wicket_attributes {
            namespace_prefix = Some(Rc::from(format!("{}:", namespace).to_owned()));
        }
        if self.get_xml_tag().has_attributes() {
            for xml_attribute in self.get_xml_tag().get_attributes().iter() {
                if namespace_prefix
                    .as_deref()
                    .is_none_or(|nsp| !xml_attribute.key_starts_with(self.tag.source(), nsp))
                {
                    //Write the attribute when it is not a wicket attribute.
                    //If it is a wicket attrib only write it when we are not stripping them.
                    let key = xml_attribute.key(self.tag.source());
                    let value = xml_attribute.value(self.tag.source());
                    response.write(" ");
                    response.write(key);
                    response.write(r#"=""#);
                    response.write(value);
                    response.write(r#"\"""#);
                }
            }
        }
        if self.get_xml_tag().tag_type().eq(&TagType::OpenClose) {
            response.write("/");
            response.write(">");
        }
    }

    /// Return the underlying xml tag.
    pub fn get_xml_tag(&self) -> &XmlTag {
        &self.tag
    }

    /// Manually mark the ComponentTag being modified. Flagging the tag being modified does not
    /// happen automatically.
    pub fn set_modified(&mut self, modified: bool) {
        if modified {
            self.flags.insert(ComponentTagFlags::MODIFIED);
        } else {
            self.flags.remove(ComponentTagFlags::MODIFIED);
        }
    }

    /// True if the ComponentTag as been marked as modified.
    pub fn is_modified(&self) -> bool {
        (self.flags & ComponentTagFlags::MODIFIED).is_empty()
    }

    /// Set true when the HTML tag (e.g. br) has no close tag.
    pub fn set_has_no_close_tag(&mut self, has_no_close_tag: bool) {
        if has_no_close_tag {
            self.flags.insert(ComponentTagFlags::NO_CLOSE_TAG);
        } else {
            self.flags.remove(ComponentTagFlags::NO_CLOSE_TAG);
        }
    }

    /// True when the HTML tag (e.g. br) has no close tag.
    pub fn is_has_no_close_tag(&self) -> bool {
        (self.flags & ComponentTagFlags::NO_CLOSE_TAG).is_empty()
    }

    /// Sets the flag to indicate if the current tag contains a child or a descendant with the
    /// "wicket::id" attribute.
    pub fn set_contains_wicket_id(&mut self, contains_wicket_id: bool) {
        if contains_wicket_id {
            self.flags.insert(ComponentTagFlags::CONTAINS_WICKET_ID);
        } else {
            self.flags.remove(ComponentTagFlags::CONTAINS_WICKET_ID);
        }
    }

    /// True when the current tag contains a child or a descendant with the "wicket::id" attribute.
    pub fn is_contains_wicket_id(&self) -> bool {
        (self.flags & ComponentTagFlags::CONTAINS_WICKET_ID).is_empty()
    }

    /// Retrieve the component containing the wicket:head tag.
    pub fn get_markup_component(&self) -> &Option<Rc<dyn Component>> {
        &self.markup_ref
    }

    /// Set the component containing the wicket:head tag.
    pub fn set_markup_component(&mut self, wicket_header_component: Option<Rc<dyn Component>>) {
        self.markup_ref = wicket_header_component;
    }

    pub fn eq_to(&self, element: &MarkupElement) -> bool {
        match &element {
            MarkupElement::ComponentTag(tag) => self.get_xml_tag().eq_xml_tag(tag.get_xml_tag()),
            _ => false,
        }
    }

    /// If true the MarkupParser will exclude it from the markup.
    pub fn is_ignore(&self) -> bool {
        (self.flags & ComponentTagFlags::IGNORE).is_empty()
    }

    /// If true the MarkupParser will exclude it from the markup.
    pub fn set_ignore(&mut self, ignore: bool) {
        if ignore {
            self.flags.insert(ComponentTagFlags::IGNORE);
        } else {
            self.flags.remove(ComponentTagFlags::IGNORE);
        }
    }

    /// True if wicket:id is automatically created (internal component).
    pub fn is_auto_component_tag(&self) -> bool {
        (self.flags & ComponentTagFlags::AUTO_COMPONENT).is_empty()
    }

    /// True if wicket:id is automatically created (internal component).
    pub fn set_auto_component_tag(&mut self, auto_component_tag: bool) {
        if auto_component_tag {
            self.flags.insert(ComponentTagFlags::AUTO_COMPONENT);
        } else {
            self.flags.remove(ComponentTagFlags::AUTO_COMPONENT);
        }
    }

    pub fn get_user_data(&self, key: &str) -> Option<&String> {
        match &self.user_data {
            Some(attrs) => attrs.get(key),
            None => None,
        }
    }

    pub fn set_user_data(&mut self, key: &str, value: &str) {
        let attrs = self
            .user_data
            .get_or_insert_with(|| HashMap::with_capacity(1));

        attrs.insert(key.to_owned(), value.to_owned());
    }
}
