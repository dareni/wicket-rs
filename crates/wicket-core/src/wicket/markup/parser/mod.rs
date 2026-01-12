use wicket_util::wicket::util::collections::io::fully_buffered_reader::ParseException;

use crate::wicket::markup::{markup_element::MarkupElement, MarkupResourceStream};

pub mod filter;
pub mod xml_pull_parser;
pub mod xml_tag;

pub trait MarkupFilter {
    /// The next markup filter(parent) in the chain.
    fn get_next_filter(&self) -> Box<&dyn MarkupFilter>;
    fn set_markup_stream(&self, _markup_stream: MarkupResourceStream) {
        unimplemented!()
    }
    fn next_element(&mut self) -> Result<Option<MarkupElement>, ParseException>;
}
