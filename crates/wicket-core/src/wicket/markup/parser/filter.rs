use crate::wicket::markup::{
    markup_element::ComponentTag,
    markup_parser::{WICKET, WICKET_ID},
};
use wicket_util::wicket::util::collections::io::fully_buffered_reader::FullyBufferedReader;

use crate::wicket::markup::{
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
            }

            if wicket_id.is_some() {
                if ct.id_str().is_none_or(|x| x.is_empty()) {
                    let position = ct.tag.text_range.start;
                    let (line, column) =
                        FullyBufferedReader::count_lines_in_str(&ct.tag.source()[0..position]);

                    return Err(WicketException::EmptyWicketId {
                        line,
                        column,
                        position,
                    });
                }
                let wicket_tag = ct.enable_wicket();
                wicket_tag.id = wicket_id;
            }
        }
        Ok(FilterResult::Keep(Box::new(element)))
    }
}

impl WicketTagIdentifier {
    pub fn is_well_known(&self, _tag: &ComponentTag) -> bool {
        unimplemented!();
    }
    pub fn is_raw(&self, _tag: &ComponentTag) -> bool {
        unimplemented!();
    }
}
