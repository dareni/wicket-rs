use wicket_util::wicket::util::collections::io::fully_buffered_reader::ParseException;

use crate::wicket::markup::markup_element::{ComponentTag, MarkupElement, SpecialTag};
use crate::wicket::markup::parser::xml_pull_parser::HttpTagType;
use crate::wicket::markup::{markup_parser::XmlPullParser, parser::MarkupFilter};

pub struct RootMarkupFilter {
    xml_pull_parser: XmlPullParser,
}
impl RootMarkupFilter {
    pub fn new(xml_pull_parser: XmlPullParser) -> Self {
        Self { xml_pull_parser }
    }
}

impl MarkupFilter for RootMarkupFilter {
    fn get_next_filter(&self) -> Box<&dyn MarkupFilter> {
        todo!()
    }

    fn next_element(&mut self) -> Result<Option<MarkupElement>, ParseException> {
        loop {
            match self.xml_pull_parser.next_iteration()? {
                HttpTagType::NotInitialized => return Ok(Option::<MarkupElement>::None),
                HttpTagType::Body => continue,
                HttpTagType::Tag => {
                    let component_tag =
                        ComponentTag::from_xml_tag(self.xml_pull_parser.get_element().unwrap());
                    return Ok(Some(MarkupElement::ComponentTag(component_tag)));
                }
                _ => {
                    let tag = self.xml_pull_parser.get_element();
                    return match tag {
                        Some(xml_tag) => {
                            Ok(Some(MarkupElement::SpecialTag(SpecialTag { tag: xml_tag })))
                        }
                        None => Ok(Option::<MarkupElement>::None),
                    };
                }
            }
        }
    }
}

pub struct HtmlHandler {}
impl HtmlHandler {
    pub fn requires_close_tag(_name: &str) -> bool {
        //TODO:
        todo!("Complete impl.");
    }
}
impl MarkupFilter for HtmlHandler {
    fn get_next_filter(&self) -> Box<&dyn MarkupFilter> {
        todo!()
    }

    fn next_element(&mut self) -> std::result::Result<Option<MarkupElement>, ParseException> {
        todo!()
    }
}
