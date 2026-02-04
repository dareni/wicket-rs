pub mod loader;
pub mod markup_element;
pub mod markup_parser;
pub mod parser;

use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::io::{self, Write};
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

    /// Render from here.
    /// Move away from distributed render MarkupStream, MarkupContainer, MarkupResponse
    pub fn render<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for element in &self.elements {
            match element {
                // The "Super-Slice" Win: High-speed block copy
                MarkupElement::RawMarkup(raw) => {
                    writer.write_all(self.source[raw.text_range.clone()].as_bytes())?;
                }

                // The Dynamic Part:
                MarkupElement::ComponentTag(tag) => {
                    if tag.wicket.is_some() {
                        // It's a non-wicket tag that was modified
                        // Render it directly and continue
                        writer.write_all(tag.tag.to_xml_string().as_bytes())?;
                    } else {
                        // It's a real Wicket Component
                        // Find the component in the page hierarchy
                        // Call component.render(tag, writer)
                        writer.write_all([b'<'].as_ref())?;
                        writer.write_all(self.source[tag.tag.pos()..].as_bytes())?;
                        let _clone = tag.shadow_copy();
                        //TODO: Let each component render it's dynamic content.
                        //component_registry.render_component(wicket_id, clone, writer)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
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
