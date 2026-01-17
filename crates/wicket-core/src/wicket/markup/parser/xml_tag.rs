use std::fmt;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use smallvec::SmallVec;
use wicket_util::wicket::util::collections::io::fully_buffered_reader::{
    FullyBufferedReader, ParseException,
};

/// The three possible tag kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagType {
    Close,
    Open,
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

#[derive(Debug)]
pub enum AttrValue {
    /// Zero-copy: just the coordinates in the Arc<str>.
    Raw(Range<usize>),
    /// Processed: the unescaped result.
    Unescaped(String),
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

pub struct XmlTag {
    /// The entire xml source containing this tag.
    source: Arc<str>,
    /// The range of the entire tag: e.g., `<wicket:label id="test">`.
    text_range: Range<usize>, //
    tag_type: TagType,
    pub name_range: Range<usize>,
    pub namespace_range: Option<Range<usize>>,

    /// Attributes: HashMap<Range<usize>, AttrValue>,
    attributes: SmallVec<[XmlAttribute; 4]>,
    /// The open tag this close tag matches.
    closes: Option<Rc<XmlTag>>,
}
impl Default for XmlTag {
    fn default() -> Self {
        Self {
            source: Arc::default(),
            text_range: Range::default(),
            tag_type: TagType::Open,
            name_range: Range::default(),
            namespace_range: None,
            attributes: SmallVec::new(),
            closes: None,
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
            name_range,
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
}

impl XmlTag {
    // -------------------------------------------------------------------------
    //  Basic getters
    // -------------------------------------------------------------------------
    pub fn tag_type(&self) -> TagType {
        self.tag_type
    }
    pub fn is_close(&self) -> bool {
        self.tag_type == TagType::Close
    }
    pub fn is_open(&self) -> bool {
        self.tag_type == TagType::Open
    }
    pub fn is_open_close(&self) -> bool {
        self.tag_type == TagType::OpenClose
    }

    pub fn text(&self) -> &str {
        &self.source[self.text_range.clone()]
    }

    pub fn name(&self) -> &str {
        &self.source[self.name_range.clone()]
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace_range
            .as_ref()
            .map(|r| &self.source[r.clone()])
    }

    pub fn get_line_and_column(&self) -> (usize, usize) {
        FullyBufferedReader::count_lines_in_str(&self.source)
    }

    pub fn pos(&self) -> usize {
        self.text_range.start
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
    pub fn set_open_tag(&mut self, open: Rc<XmlTag>) {
        self.closes = Some(open);
    }

    pub fn get_open_tag(&self) -> Option<&XmlTag> {
        self.closes.as_deref()
    }

    pub fn closes(&self, open: &XmlTag) -> bool {
        let val = self.closes.as_ref();
        match val {
            Some(rc_val) => std::ptr::eq(open, rc_val.as_ref() as &XmlTag),
            None => false,
        }
    }

    // -------------------------------------------------------------------------
    //  Debug / string conversion
    // -------------------------------------------------------------------------
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
        buf.push_str(self.name());

        for attrib in &self.attributes {
            let key = &self.source[attrib.key_range.clone()];
            let value: &str = match &attrib.value {
                AttrValue::Raw(range) => &self.source[range.clone()],
                AttrValue::Unescaped(str) => str.as_ref(),
            };
            buf.push(' ');
            buf.push_str(key);
            buf.push_str("=\"");
            buf.push_str(value);
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
    use super::*;

    #[test]
    fn test_closes() {
        let close_tag = Rc::from(XmlTag::default());
        let alt_close_tag = Rc::from(XmlTag::default());

        let xml_tag = XmlTag {
            closes: Some(close_tag.clone()),
            ..Default::default()
        };
        assert!(xml_tag.closes(&close_tag));
        assert!(!xml_tag.closes(&alt_close_tag));
    }
}
