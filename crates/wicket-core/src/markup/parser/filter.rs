use crate::markup::{
    markup_element::ComponentTag,
    markup_parser::{WICKET, WICKET_ID},
};
use wicket_util::collections::io::fully_buffered_reader::FullyBufferedReader;

use crate::markup::{
    markup_element::MarkupElement,
    parser::{xml_tag::AttrValue, WicketException},
    MarkupResourceStream,
};

pub trait MarkupFilter {
    fn set_markup_stream(&self, _markup_stream: MarkupResourceStream) {
        unimplemented!()
    }

    /// Process in turn from the list of filters instead  of from the chain.
    /// fyi replaces onComponentTag(), onSpecialTag().
    fn process(&mut self, element: MarkupElement) -> Result<FilterResult, WicketException>;
}
pub struct HtmlHandler {}
impl HtmlHandler {
    pub fn requires_close_tag(_name: &str) -> bool {
        //TODO:
        todo!("Complete impl.");
    }
}
impl MarkupFilter for HtmlHandler {
    fn process(&mut self, _element: MarkupElement) -> Result<FilterResult, WicketException> {
        todo!()

        //Ok(FilterResult::Keep(Box::new(*element)))
    }
}

/// from org.apache.wicket.markup.parser.filter.WicketTagIdentifier
const WELL_KNOWN_TAG_NAMES: [&str; 14] = [
    "border",
    "body",
    "label",
    "panel",
    "enclosure",
    "link",
    "remove",
    "fragment",
    "head",
    "header-items",
    "child",
    "extend",
    "container",
    "message",
];

pub enum FilterResult {
    /// Keep this element and pass it to the next filter
    Keep(Box<MarkupElement>),
    /// Drop this element (effectively deleting it from the stream)
    Drop,
    /// Replace this element with multiple elements (Expansion)
    /// Example: <div/> becomes <div> and </div>
    Replace(Vec<MarkupElement>),
}

pub struct WicketTagIdentifier {}

impl MarkupFilter for WicketTagIdentifier {
    fn process(&mut self, mut element: MarkupElement) -> Result<FilterResult, WicketException> {
        if let MarkupElement::ComponentTag(ref mut ct) = element {
            let wicket_id = ct.tag.get_attribute_attrvalue(WICKET_ID);

            if let Some(id) = &wicket_id {
                if id.to_str(ct.tag.source()).is_empty() {
                    let position = ct.tag.text_range.start;
                    let (line, column) =
                        FullyBufferedReader::count_lines_in_str(&ct.tag.source()[0..position]);

                    return Err(WicketException::EmptyWicketId {
                        line,
                        column,
                        position,
                    });
                }
            }

            if ct
                .tag
                .namespace()
                .is_some_and(|ns| ns.eq_ignore_ascii_case(WICKET))
            {
                if wicket_id.is_none() {
                    let tmp_id = format!(
                        "{}_{}{}",
                        WICKET,
                        ct.tag.name(),
                        MarkupElement::get_request_unique_id()
                    );
                    let wicket_tag = ct.enable_wicket();
                    wicket_tag.id = Some(AttrValue::Unescaped(tmp_id));
                }
                if !self.is_well_known(ct) {
                    let position = ct.tag.text_range.start;
                    let (line, column) =
                        FullyBufferedReader::count_lines_in_str(&ct.tag.source()[0..position]);

                    return Err(WicketException::UnknownTag {
                        name: ct.tag.name().into_owned(),
                        line,
                        column,
                        position,
                    });
                }
            } else if wicket_id.is_some() {
                let wicket_tag = ct.enable_wicket();
                wicket_tag.id = wicket_id.clone();
            }
        }
        Ok(FilterResult::Keep(Box::new(element)))
    }
}

impl WicketTagIdentifier {
    pub fn is_well_known(&self, tag: &ComponentTag) -> bool {
        WELL_KNOWN_TAG_NAMES.contains(&tag.tag.name().to_lowercase().as_str())
    }
    pub fn is_raw(&self, _tag: &ComponentTag) -> bool {
        unimplemented!();
    }
}
