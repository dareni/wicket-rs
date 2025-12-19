use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

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

impl TextSegment {
    pub fn new(text: Option<Rc<str>>, pos: usize, line: usize, col: usize) -> Self {
        Self {
            text,
            pos,
            line_number: line,
            column_number: col,
        }
    }
    pub fn len(&self) -> usize {
        self.text.as_ref().map_or(0, |t| t.len())
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_none()
    }
}

#[derive(Clone)]
pub struct XmlTag {
    // ---- immutable core data -------------------------------------------------
    text: Option<Rc<TextSegment>>, // `None` after `make_immutable`
    tag_type: TagType,
    pub name: Rc<str>,
    pub namespace: Option<Rc<str>>,

    // ---- mutable state -------------------------------------------------------
    attributes: HashMap<Rc<str>, Rc<str>>, // attribute map (String → String)
    closes: Option<Rc<XmlTag>>,            // the open tag this close tag matches
    copy_of: Option<Rc<XmlTag>>,           // immutable source of a mutable copy
    mutable: bool,                         // true = mutable, false = immutable
}
impl Default for XmlTag {
    fn default() -> Self {
        Self {
            text: None,
            tag_type: TagType::Open,
            name: Rc::from(""),
            namespace: None,
            attributes: HashMap::new(),
            closes: None,
            copy_of: None,
            mutable: true,
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

    /// Tag with a `TextSegment` and a `TagType`.
    pub fn with_text(text: TextSegment, tag_type: TagType) -> Self {
        Self {
            text: Some(Rc::new(text)),
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

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    pub fn line_number(&self) -> usize {
        self.text.as_ref().map_or(0, |t| t.line_number)
    }
    pub fn column_number(&self) -> usize {
        self.text.as_ref().map_or(0, |t| t.column_number)
    }
    pub fn pos(&self) -> usize {
        self.text.as_ref().map_or(0, |t| t.pos)
    }
    pub fn length(&self) -> usize {
        self.text.as_ref().map_or(0, |t| t.len())
    }

    // -------------------------------------------------------------------------
    //  Attribute handling
    // -------------------------------------------------------------------------

    pub fn get_attributes(&self) -> &HashMap<Rc<str>, Rc<str>> {
        &self.attributes
    }

    pub fn get_attribute(&self, key: &str) -> Option<&str> {
        // self.attributes.get(key.as_ref()).map(|v| v.as_ref())
        self.attributes.get(key).map(|s| s.as_ref())
    }

    /*
    self.attributes.get(key.as_ref()).map(|v| v.as_ref()) gives the error:

    type annotations needed
    impl`s satisfying `str: std::convert::AsRef<_>` found in the following crates: `core`, `std`:

    where attributes is:  attributes: HashMap<Rc<str>, Rc<str>>,   // attribute map (String → String)

    */

    pub fn put_attribute(
        &mut self,
        key: impl Into<Rc<str>>,
        value: impl Into<Rc<str>>,
    ) -> Option<Rc<str>> {
        self.ensure_mutable();
        self.attributes.insert(key.into(), value.into())
    }

    pub fn put_bool(&mut self, key: impl Into<Rc<str>>, value: bool) -> Option<Rc<str>> {
        self.put_attribute(key, if value { "true" } else { "false" })
    }

    pub fn put_int(&mut self, key: impl Into<Rc<str>>, value: i32) -> Option<Rc<str>> {
        self.put_attribute(key, value.to_string())
    }

    pub fn remove_attribute(&mut self, key: &str) -> Option<Rc<str>> {
        self.ensure_mutable();
        self.attributes.remove(key)
    }

    pub fn has_attributes(&self) -> bool {
        !self.attributes.is_empty()
    }

    // -------------------------------------------------------------------------
    //  Mutability control
    // -------------------------------------------------------------------------
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    /// Makes the tag immutable – clears the `text` field and prevents further mutation.
    pub fn make_immutable(&mut self) {
        if self.mutable {
            self.mutable = false;
            self.text = None; // drop the raw markup
        }
    }

    /// Returns a mutable copy if the current instance is immutable.
    pub fn mutable(&self) -> Self {
        if self.mutable {
            self.clone()
        } else {
            let mut copy = self.clone();
            copy.mutable = true;
            copy.copy_of = Some(Rc::new(self.clone()));
            copy
        }
    }

    // -------------------------------------------------------------------------
    //  Open/close linking
    // -------------------------------------------------------------------------
    pub fn set_open_tag(&mut self, open: Rc<XmlTag>) {
        self.ensure_mutable();
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
        if let Some(ns) = &self.namespace {
            buf.push_str(ns);
            buf.push(':');
        }
        buf.push_str(&self.name);
        for (k, v) in &self.attributes {
            buf.push(' ');
            buf.push_str(k);
            buf.push_str("=\"");
            buf.push_str(v);
            buf.push('"');
        }
        if self.is_open_close() {
            buf.push('/');
        }
        buf.push('>');
        buf
    }

    pub fn to_debug_string(&self) -> String {
        format!(
            "[Tag name = {}, pos = {}, line = {}, attributes = {:?}, type = {:?}]",
            self.name,
            self.pos(),
            self.line_number(),
            self.attributes,
            self.tag_type
        )
    }

    // -------------------------------------------------------------------------
    //  Internal helpers
    // -------------------------------------------------------------------------
    fn ensure_mutable(&self) {
        if !self.mutable {
            panic!("Attempt to modify an immutable XmlTag");
        }
    }

    pub fn eq_xml_tag(&self, other: &XmlTag) -> bool {
        // ---- namespace comparison (both `Option<Rc<str>>`) ----
        let ns_eq = match (&self.namespace, &other.namespace) {
            (Some(a), Some(b)) => a.as_ref() == b.as_ref(),
            (None, None) => true,
            _ => false,
        };

        if !ns_eq {
            return false;
        }

        // ---- name comparison (both `Rc<str>`) ----
        if self.name.as_ref() != other.name.as_ref() {
            return false;
        }

        // ---- attribute map comparison ----
        // The attribute maps store `Rc<str>` keys/values; we compare the underlying strings.
        if self.attributes.len() != other.attributes.len() {
            return false;
        }

        for (k, v) in &self.attributes {
            match other.attributes.get(k) {
                Some(other_v) if other_v.as_ref() == v.as_ref() => {}
                _ => return false,
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
            .field("name", &self.name)
            .field("namespace", &self.namespace)
            .field("type", &self.tag_type)
            .field("attributes", &self.attributes)
            .field("mutable", &self.mutable)
            .finish()
    }
}

#[cfg(test)]
mod tes {
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
