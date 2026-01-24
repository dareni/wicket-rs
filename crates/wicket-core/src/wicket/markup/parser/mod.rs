use crate::wicket::markup::{markup_element::MarkupElement, MarkupResourceStream};

pub mod filter;
pub mod xml_pull_parser;
pub mod xml_tag;

pub trait MarkupFilter {
    fn set_markup_stream(&self, _markup_stream: MarkupResourceStream) {
        unimplemented!()
    }

    /// Process in turn from the list of filters instead  of from the chain.
    fn process(&mut self, element: &mut MarkupElement) -> FilterResult;
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
