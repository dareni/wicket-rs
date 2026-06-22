pub mod dimensions;
pub mod loader;
pub mod markup_element;
pub mod markup_parser;
pub mod parser;

use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::Read;
use std::sync::OnceLock;

use once_cell::sync::Lazy;

use wicket_util::parse::metapattern::core::Pattern;
use wicket_util::static_pattern;

use crate::components::ComponentId;
use crate::components::MarkupContainer;
use crate::markup::loader::{DefaultMarkupResourceStreamProvider, MarkupResourceStreamProvider};
use crate::markup::markup_element::MarkupElement;
use crate::markup::markup_parser::MarkupParser;
use crate::request::Response;

static_pattern!(
    CONDITIONAL_COMMENT_OPENING,
    r"(s?)^[^>]*?<!--\[if.*?\]>(-->)?(<!.*?-->)?"
);
static_pattern!(DOCTYPE_REGEX, r"!DOCTYPE\s+(.*)\s*");

pub const WICKET_XHTML_DTD: &str = "http://wicket.apache.org/dtds.data/wicket-xhtml1.4-strict.dtd";

pub struct Markup {
    elements: OnceLock<Vec<MarkupElement>>,
    pub source: &'static str,
}

impl Default for Markup {
    fn default() -> Self {
        Self {
            elements: OnceLock::new(),
            source: "",
        }
    }
}
impl Markup {
    ///TODO: implement Markup
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn new_source(source: &'static str) -> Self {
        Self {
            elements: OnceLock::new(),
            source,
        }
    }

    pub fn get_elements(&self) -> &Vec<MarkupElement> {
        // TODO: Add parameters for dimensions and MarkupContainer strings for parse error
        // message context.
        // TODO: Long term move the parse to the proc macro for compile time processing.
        self.elements.get_or_init(|| {
            //TODO: Refactor MarkupParser::new to take a static string reference.
            let mut parser = MarkupParser::new(self.source.to_string());
            let result = parser.parse_markup();
            match result {
                Ok(v) => v,
                Err(e) => panic!("Error: could not parse markup, error: {}", e),
            }
        })
    }

    // Pulled on WebPage creation to build a map of wicket MarkupElements and
    // their Component counterparts.
    pub fn get_component_tags(&self) -> Vec<u16> {
        self.get_elements()
            .iter()
            .filter_map(|me| match me {
                MarkupElement::ComponentTag(ct) => Some(ct.tag_id),
                _ => None,
            })
            .collect()
    }

    /// Render from here.
    /// Move away from distributed render MarkupStream, MarkupContainer, MarkupResponse
    pub fn render<T: MarkupContainer>(
        &self,
        response: &mut Response,
        markup_container: &T,
    ) -> io::Result<()> {
        for element in self.get_elements() {
            match element {
                // The "Super-Slice" Win: High-speed block copy
                MarkupElement::RawMarkup(raw) => {
                    response.write_str(&self.source[raw.text_range.clone()])?;
                }

                // The Dynamic Part:
                MarkupElement::ComponentTag(tag) => {
                    if tag.wicket.is_some() {
                        // It's a non-wicket tag that was modified
                        // Render it directly and continue
                        response.write_str(&tag.tag.to_xml_string())?;
                    } else {
                        // It's a real Wicket Component
                        // Find the component in the page hierarchy
                        // Call component.render(tag, writer)
                        response.write_str("<")?;
                        response.write_str(&self.source[tag.tag.pos()..])?;
                        let _clone = tag.shadow_copy();
                        //TODO: Let each component render it's dynamic content.
                        markup_container
                            .render_component(ComponentId::TagId(tag.tag_id), response)?;
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
        let el = self.markup.get_elements().get(self.current_index);
        self.current_index += 1;
        el
    }

    pub fn is_current_tag(&self) -> bool {
        matches!(
            self.markup.get_elements().get(self.current_index),
            Some(MarkupElement::ComponentTag(_))
        )
    }
}

#[derive(Default)]
pub struct MarkupFactory {}

impl MarkupFactory {
    pub fn get_markup_resource_provider() -> Box<dyn MarkupResourceStreamProvider> {
        Box::from(DefaultMarkupResourceStreamProvider::new_default())
    }
}

/// style, variation, lang, country index to ValidHtmlDimensions
/// In Apache Wicket, HTML files are resolved using the ResourceStreamLocator class,
/// which combines the component's variation, the session's style, and the thread's locale.
pub struct MarkupResource {
    pub style: Option<u8>,
    pub variation: Option<u8>,
    pub lang: Option<u8>,
    pub country: Option<u8>,
    pub markup: Markup,
}

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
