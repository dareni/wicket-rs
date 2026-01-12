pub mod loader;
pub mod markup_element;
pub mod markup_parser;
pub mod parser;

use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::sync::Arc;
use wicket_util::wicket::util::parse::metapattern::Pattern;

use crate::wicket::markup::markup_element::MarkupElement;
use wicket_util::static_pattern;

static_pattern!(
    CONDITIONAL_COMMENT_OPENING,
    r"(s?)^[^>]*?<!--\[if.*?\]>(-->)?(<!.*?-->)?"
);
static_pattern!(DOCTYPE_REGEX, r"!DOCTYPE\s+(.*)\s*");

pub const WICKET_XHTML_DTD: &str = "http://wicket.apache.org/dtds.data/wicket-xhtml1.4-strict.dtd";

pub struct Markup {
    pub elements: Vec<MarkupElement>,
    pub source: Arc<str>,
    pub markup_resource_stream: MarkupResourceStream,
}

impl Default for Markup {
    fn default() -> Self {
        Self {
            elements: vec![],
            source: Arc::from("".to_string().into_boxed_str()),
            markup_resource_stream: MarkupResourceStream {},
        }
    }
}
impl Markup {
    ///TODO: implement Markup
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct MarkupStream<'a> {
    markup: &'a Markup,
    current_index: usize,
}

impl<'a> MarkupStream<'a> {
    pub fn next_element(&mut self) -> Option<&MarkupElement> {
        let el = self.markup.elements.get(self.current_index);
        self.current_index += 1;
        el
    }

    pub fn is_current_tag(&self) -> bool {
        matches!(
            self.markup.elements.get(self.current_index),
            Some(MarkupElement::ComponentTag(_))
        )
    }
}

#[derive(Default)]
pub struct MarkupFactory {}

#[derive(Default)]
pub struct MarkupResourceStream {}
