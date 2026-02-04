use crate::wicket::markup::{markup_element::MarkupElement, MarkupResourceStream};
use thiserror::Error;

pub mod filter;
pub mod xml_pull_parser;
pub mod xml_tag;

#[derive(Debug, Error)]
pub enum WicketException {
    #[error("wicket processing failed due to parse error: {0}")]
    Parse(#[from] ParseException),
    #[error(
        "WicketException: The wicket:id value must not be empty at (line \
        {line}, column {column}) position {position}"
    EmptyWicketId {
        line: usize,
        column: usize,
        position: usize,
    },
    #[error("The quoted value has whitespace  prepended or appended at (line {line}, column {column}) at position {position}.")]
    NoOpenTag {
        line: usize,
        column: usize,
        position: usize,
    },
    #[error("The open tag name '{open_name}' does not match the closing tag name '{close_name}' found at (line {line}, column {column}) at position {position}.")]
    UnmatchedTagName {
        close_name: String,
        open_name: String,
        line: usize,
        column: usize,
        position: usize,
    },
    #[error("The tag name '{name}' is unknown to wicket, at (line {line}, column {column}) at position {position}.")]
    UnknownTag {
        name: String,
        line: usize,
        column: usize,
        position: usize,
    },
}

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
