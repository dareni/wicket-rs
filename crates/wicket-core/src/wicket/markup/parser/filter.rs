use crate::wicket::markup::markup_element::MarkupElement;
use crate::wicket::markup::parser::MarkupFilter;

pub struct HtmlHandler {}
impl HtmlHandler {
    pub fn requires_close_tag(_name: &str) -> bool {
        //TODO:
        todo!("Complete impl.");
    }
}
impl MarkupFilter for HtmlHandler {
    fn process(&mut self, element: &mut MarkupElement) -> super::FilterResult {
        todo!()
    }
}
