use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::io::{self, Write};
use std::sync::Arc;
use wicket_util::wicket::util::parse::metapattern::Pattern;

use crate::wicket::core::markup::markup_element::MarkupElement;
use wicket_util::static_pattern;

pub mod loader;
pub mod markup_element;
pub mod markup_parser;
pub mod parser;

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
            markup_resource_stream: MarkupResourceStream { variation: None },
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

/// A stream of MarkupElement. A markup stream has a current index in the list of markup elements.
/// The next markup element can be retrieved and the index advanced by calling next(). If the
/// index hits the end, hasMore() will return false.
///
//  A component of the render machinery, MarkupStream is the "Script" and the Component Tree
//  is the "Actor." The Actor follows the Script line-by-line, but the Actor decides how, or if
//  those lines are spoken.
//
///  = The current markup element can be accessed with get() and as a ComponentTag with getTag().
///  = The stream can be sought to a particular location with setCurrentIndex().
///
/// Convenience methods also exist to skip component tags (and any potentially nested markup) or raw
/// markup.
///
/// Several boolean methods of the form at*() return true if the markup stream is positioned at a tag
/// with a given set of characteristics.
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

pub trait ResourceStream {
    fn get_variation(&self) -> Option<&str> {
        None
    }
    fn get_read(&mut self) -> &mut dyn Read;
}

#[derive(Default)]
pub struct MarkupResourceStream {
    variation: Option<String>,
}
impl ResourceStream for MarkupResourceStream {
    fn get_variation(&self) -> Option<&str> {
        self.variation.as_deref()
    }

    fn get_read(&mut self) -> &mut dyn Read {
        todo!()
    }
}

/// Access to the resource raw bytes with web metadata.
pub struct FileResourceStream {
    pub file: File,
    pub variation: Option<String>,
}
impl ResourceStream for FileResourceStream {
    fn get_variation(&self) -> Option<&str> {
        self.variation.as_deref()
    }

    fn get_read(&mut self) -> &mut dyn Read {
        &mut self.file as &mut dyn Read
    }
}
