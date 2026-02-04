use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::{borrow::Cow, fmt};

use smallvec::SmallVec;

use wicket_util::wicket::util::collections::io::fully_buffered_reader::{
    FullyBufferedReader, ParseException,
};
use wicket_util::wicket::util::string::strings::escape_markup;

use crate::wicket::markup::markup_element::{ComponentTag, MarkupElement};

/// The three possible tag kinds.
/// Store an index into Markup.components for the relative tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagType {
    Close { opener_index: Option<usize> },
    Open { closer_index: Option<usize> },
    OpenClose,
}

/// Holds the raw text of a tag together with its position in the source markup.
#[derive(Debug, Clone)]
pub struct TextSegment {
    pub column_number: usize,
    pub line_number: usize,
    pub pos: usize,
    pub text: Option<Rc<str>>, // shared, immutable string slice
}

#[derive(Debug, Clone)]
pub enum AttrValue {
    /// Zero-copy: just the coordinates in the Arc<str>.
    Raw(Range<usize>),
    /// Processed: the unescaped result.
    Unescaped(String),
}

impl AttrValue {
    pub fn to_str<'a>(&'a self, source: &'a str) -> &'a str {
        match &self {
            AttrValue::Raw(range) => &source[range.clone()],
            AttrValue::Unescaped(unescaped) => unescaped.as_str(),
        }
    }
}

#[derive(Debug)]
pub struct XmlAttribute {
    pub key_range: Range<usize>,
    pub value: AttrValue,
}

impl XmlAttribute {
    pub fn equals(&self, other: &XmlAttribute) -> bool {
        if self.key_range != other.key_range {
            return false;
        }
        match (&self.value, &other.value) {
            (AttrValue::Raw(range_s), AttrValue::Raw(range_o)) => range_s == range_o,
            (AttrValue::Unescaped(str_s), AttrValue::Unescaped(str_o)) => str_s == str_o,
            _ => false,
        }
    }

    pub fn key<'a>(&self, source: &'a str) -> &'a str {
        &source[self.key_range.clone()]
    }

    pub fn value<'a>(&'a self, source: &'a str) -> &'a str {
        match &self.value {
            AttrValue::Raw(range) => &source[range.clone()],
            AttrValue::Unescaped(unescaped) => unescaped.as_str(),
        }
    }

    pub fn eq_key(&self, source: &str, other_key: &str) -> bool {
        if self.key_range.len() != other_key.len() {
            return false;
        }
        &source[self.key_range.clone()] == other_key
    }

    pub fn key_starts_with(&self, source: &str, prefix: &str) -> bool {
        if self.key_range.len() < prefix.len() {
            return false;
        }
        source[self.key_range.clone()].starts_with(prefix)
    }
}

/// Use as much of the original xml markup as possible, any changes to the original are stored as
/// required.
#[derive(Debug)]
pub enum XmlString {
    /// Points to the original HTML source.
    Raw(Range<usize>),
    /// The variant used for changes made by MarkupFilters.
    Modified(Arc<String>),
    /// The Component request-local modifications.
    Dynamic(String),
}

impl XmlString {
    ///The Cow makes the value available even when the source is not.
    pub fn value<'a>(&'a self, source: &'a str) -> Cow<'a, str> {
        match self {
            Self::Raw(range) => Cow::Borrowed(&source[range.clone()]),
            Self::Modified(modified_str) => Cow::Borrowed(&**modified_str),
            Self::Dynamic(request_local_string) => Cow::Owned(request_local_string.to_owned()),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Raw(range) => range.len(),
            Self::Modified(modified_str) => modified_str.len(),
            Self::Dynamic(request_local_string) => request_local_string.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Raw(range) => range.is_empty(),
            Self::Modified(modified_str) => modified_str.is_empty(),
            Self::Dynamic(request_local_string) => request_local_string.is_empty(),
        }
    }
}

pub struct XmlTag {
    /// The entire xml source containing this tag.
    source: Arc<str>,
    /// The range of the entire tag: e.g., `<wicket:label id="test">`.
    pub text_range: Range<usize>, //
    /// Also contains the index to the open/close relative.
    tag_type: TagType,
    pub name_range: XmlString,
    pub namespace_range: Option<XmlString>,

    /// Attributes: HashMap<Range<usize>, AttrValue>,
    attributes: SmallVec<[XmlAttribute; 4]>,
    /// render entirely from the source when the tag is unmodified.
    modified: bool,
}
impl Default for XmlTag {
    fn default() -> Self {
        Self {
            source: Arc::default(),
            text_range: Range::default(),
            tag_type: TagType::Open {
                closer_index: Some(0),
            },
            name_range: XmlString::Raw(Range::default()),
            namespace_range: None,
            attributes: SmallVec::new(),
            modified: false,
        }
    }
}

impl XmlTag {
    /// Empty tag – mutable by default.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_from(name_range: Range<usize>, tag_type: &TagType) -> Self {
        Self {
            name_range: XmlString::Raw(name_range),
            tag_type: tag_type.to_owned(),
            ..Default::default()
        }
    }

    /// Tag with a `TextSegment` and a `TagType`.
    pub fn with_text(source: Arc<str>, text_range: Range<usize>, tag_type: TagType) -> Self {
        Self {
            source,
            text_range,
            tag_type,
            ..Default::default()
        }
    }
    pub fn set_modified(&mut self) {
        self.modified = true;
    }
}

impl XmlTag {
    // -------------------------------------------------------------------------
    //  Basic getters
    // -------------------------------------------------------------------------
    pub fn tag_type(&self) -> TagType {
        self.tag_type
    }
    pub fn is_close(&self) -> bool {
        matches!(&self.tag_type, TagType::Close { opener_index: _ })
    }
    pub fn is_open(&self) -> bool {
        matches!(&self.tag_type, TagType::Open { closer_index: _ })
    }

    pub fn is_open_close(&self) -> bool {
        self.tag_type == TagType::OpenClose
    }

    /// The text for the tag.
    pub fn text(&self) -> &str {
        &self.source[self.text_range.clone()]
    }

    pub fn name(&self) -> Cow<'_, str> {
        self.name_range.value(&self.source)
    }
    pub fn namespace(&self) -> Option<Cow<'_, str>> {
        self.namespace_range.as_ref().map(|r| r.value(&self.source))
    }

    pub fn get_line_and_column(&self) -> (usize, usize) {
        FullyBufferedReader::count_lines_in_str(&self.source)
    }

    /// The starting postion of the tag (the less than symbol) in the original markup.
    pub fn pos(&self) -> usize {
        self.text_range.start - 1
    }
    pub fn length(&self) -> usize {
        self.text_range.len()
    }
    pub fn source(&self) -> &str {
        &self.source
    }

    // -------------------------------------------------------------------------
    //  Attribute handling
    // -------------------------------------------------------------------------

    pub fn get_attributes(&self) -> &SmallVec<[XmlAttribute; 4]> {
        &self.attributes
    }

    pub fn get_attribute_value(&self, key: &str) -> Option<&str> {
        for xml_attribute in &self.attributes {
            if xml_attribute.eq_key(&self.source, key) {
                let ret = Some(match &xml_attribute.value {
                    // Turn the Range into a borrow of our Arc
                    AttrValue::Raw(range) => &self.source[range.clone()],
                    // Return a borrow of our already-owned String
                    AttrValue::Unescaped(s) => s.as_str(),
                });
                return ret;
            }
        }
        None
    }

    pub fn put_attribute(&mut self, attrib: XmlAttribute) -> Result<(), ParseException> {
        // Check the attribute does not already exist before adding it.
        for existing_attrib in &self.attributes {
            if attrib.key_range.len() == existing_attrib.key_range.len() {
                let new_key = &self.source[attrib.key_range.clone()];
                let existing_key = &self.source[existing_attrib.key_range.clone()];
                if new_key.eq(existing_key) {
                    let (line, column) = FullyBufferedReader::count_lines_in_str(
                        &self.source[..self.text_range.clone().start],
                    );
                    let value: String = match attrib.value {
                        AttrValue::Raw(range) => self.source[range].into(),
                        AttrValue::Unescaped(str) => str,
                    };

                    return Err(ParseException::AttributeExists {
                        line,
                        column,
                        position: self.pos(),
                        tag_key: new_key.into(),
                        tag_value: value,
                    });
                }
            }
        }
        self.attributes.push(attrib);
        Ok(())
    }

    pub fn has_attributes(&self) -> bool {
        !self.attributes.is_empty()
    }

    pub fn contains_attribute_key(&self, key: &str) -> bool {
        for xml_attribute in &self.attributes {
            if xml_attribute.key_range.len() == key.len()
                && &self.source[xml_attribute.key_range.clone()] == key
            {
                return true;
            }
        }
        false
    }

    pub fn contains_attribute_value(&self, value: &str) -> bool {
        for xml_attribute in &self.attributes {
            match &xml_attribute.value {
                AttrValue::Raw(attr_range) => {
                    if attr_range.len() == value.len() && &self.source[attr_range.clone()] == value
                    {
                        return true;
                    }
                }
                AttrValue::Unescaped(unescaped) => {
                    if unescaped == value {
                        return true;
                    }
                }
            }
        }
        false
    }

    // -------------------------------------------------------------------------
    //  Open/close linking
    // -------------------------------------------------------------------------
    pub fn set_open_tag(&mut self, opener_index: Option<usize>) {
        self.tag_type = TagType::Close { opener_index };
    }

    pub fn get_open_tag<'a>(
        &self,
        open_index: usize,
        components: &'a [ComponentTag],
    ) -> Option<&'a XmlTag> {
        match components.get(open_index) {
            Some(component_tag) => Some(&component_tag.tag),
            None => None,
        }
    }

    /// Does this open tag correspond to the open tag reference stored in this close tag?
    pub fn closes(&self, open: &XmlTag, components: &[MarkupElement]) -> bool {
        match self.tag_type {
            TagType::Close {
                opener_index: Some(open_index),
            } => match components.get(open_index) {
                Some(MarkupElement::ComponentTag(component_tag)) => {
                    std::ptr::eq(&component_tag.tag, open)
                }
                _ => false,
            },
            _ => false,
        }
    }

    // -------------------------------------------------------------------------
    //  Debug / string conversion
    // -------------------------------------------------------------------------

    pub fn to_char_sequence<'a>(&'a self) -> Cow<'a, str> {
        if self.modified {
            Cow::Owned(self.to_xml_string())
        } else {
            Cow::Borrowed(self.text())
        }
    }

    pub fn to_xml_string(&self) -> String {
        let mut buf = String::new();
        buf.push('<');
        if self.is_close() {
            buf.push('/');
        }
        if let Some(ns) = &self.namespace() {
            buf.push_str(ns);
            buf.push(':');
        }
        buf.push_str(self.name().as_ref());

        for attrib in &self.attributes {
            let key = &self.source[attrib.key_range.clone()];
            buf.push(' ');
            buf.push_str(key);
            buf.push_str("=\"");

            match &attrib.value {
                AttrValue::Raw(range) => buf.push_str(&self.source[range.clone()]),
                AttrValue::Unescaped(str) => {
                    //let escaped: Cow<'_, str> = escape_markup(str);
                    let escaped = escape_markup(str);
                    buf.push_str(escaped.as_ref());
                }
            };

            buf.push('"');
        }
        if self.is_open_close() {
            buf.push('/');
        }
        buf.push('>');
        buf
    }

    pub fn to_debug_string(&self) -> String {
        let (line_number, _) = self.get_line_and_column();
        format!(
            "[Tag name = {}, pos = {}, line = {}, attributes = {:?}, type = {:?}]",
            self.name(),
            self.pos(),
            line_number,
            self.attributes,
            self.tag_type
        )
    }

    // -------------------------------------------------------------------------
    //  Internal helpers
    // -------------------------------------------------------------------------

    pub fn eq_xml_tag(&self, other: &XmlTag) -> bool {
        // ---- namespace comparison (both `Option<Rc<str>>`) ----
        let ns_eq = match (&self.namespace(), &other.namespace()) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };

        if !ns_eq {
            return false;
        }

        // ---- name comparison (both `Rc<str>`) ----
        if self.name() != other.name() {
            return false;
        }

        // ---- attribute map comparison ----
        // The attribute maps store `Rc<str>` keys/values; we compare the underlying strings.
        if self.attributes.len() != other.attributes.len() {
            return false;
        }

        for idx in 0..self.attributes.len() {
            let attrib_o = self.attributes.get(idx);
            let other_attrib_o = other.attributes.get(idx);

            match (attrib_o, other_attrib_o) {
                (Some(attrib), Some(other_attrib)) => {
                    if !attrib.equals(other_attrib) {
                        return false;
                    }
                }
                (None, None) => {}
                _ => {
                    return false;
                }
            }
        }

        true
    }
}

impl fmt::Display for XmlTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_xml_string())
    }
}

impl fmt::Debug for XmlTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XmlTag")
            .field("name", &self.name())
            .field("namespace", &self.namespace())
            .field("type", &self.tag_type)
            .field("attributes", &self.attributes)
            .finish()
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_closes() {}
}
