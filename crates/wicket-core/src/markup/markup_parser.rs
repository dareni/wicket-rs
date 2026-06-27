use std::collections::VecDeque;
use std::{borrow::Cow, io::Read};

use once_cell::sync::Lazy;

use crate::markup::parser::filter::WicketTagIdentifier;
use crate::markup::parser::xml_tag::{TagType, XmlString};
use crate::{
    markup::{
        markup_element::{ComponentTag, MarkupElement, RawMarkup, SpecialTag},
        parser::{
            filter::{FilterResult, MarkupFilter},
            xml_pull_parser::{HttpTagType, XmlPullParser},
            WicketException,
        },
        Markup,
    },
    settings::MarkupSettings,
};
use wicket_util::parse::metapattern::core::Pattern;
use wicket_util::{
    collections::io::fully_buffered_reader::{FullyBufferedReader, ParseException},
    parse::metapattern::core::RegexFlags,
    static_pattern,
};

/// The wicket namespace, hardcoded for simplicity, will anyone care?
pub static WICKET_ID: &str = "wicket:id";
pub static WICKET: &str = "wicket";

// Opening a conditional comment section, which is NOT treated as a comment section
static_pattern!(
    CONDITIONAL_COMMENT_OPENING,
    r"(s?)^[^>]*?<!--\[if.*?\]>(-->)?(<!.*?-->)?"
);

pub static PRE_BLOCK: Lazy<Pattern> = Lazy::new(|| {
    Pattern::new_with_flags(
        r"<pre>.*?</pre>".into(),
        &RegexFlags::DOT_MATCHES_NEW_LINE.union(RegexFlags::MULTI_LINE),
    )
});

static_pattern!(SPACE_OR_TAB_PATTERN, r"[ \\t]+");
static_pattern!(NEW_LINE_PATTERN, r"( ?[\\r\\n] ?)+");

pub struct MarkupParser {
    pub xml_parser: XmlPullParser,
    // The markup handler chain: each filter has a specific task.
    pub markup_filter_chain: Vec<Box<dyn MarkupFilter>>,
    // The maarkup created from the input markup file.
    pub markup: Markup,
    pub markup_settings: MarkupSettings,
    // Temporary filter storage for related MarkupElements.
    pub queue: VecDeque<MarkupElement>,
}

impl Default for MarkupParser {
    fn default() -> Self {
        //TODO: engineer a mechanism to allow filter chain configuration.
        let mut markup_filter_chain: Vec<Box<dyn MarkupFilter>> = Vec::with_capacity(10);
        markup_filter_chain.push(Box::new(WicketTagIdentifier {}));
        Self {
            xml_parser: Default::default(),
            markup_filter_chain,
            markup: Markup::new(),
            markup_settings: MarkupSettings::default(),
            queue: Default::default(),
        }
    }
}

// TODO: set markup range tyes to u16. 64.5KB sized html file max.
enum TagPairing {
    WicketTag(usize),
    Raw { tag_type: TagType, name: XmlString },
}

impl MarkupParser {
    //TODO: Add postProcess() and filter chain configuration.

    pub fn new(input: String) -> Self {
        Self {
            xml_parser: XmlPullParser::new(input),
            ..Default::default()
        }
    }

    pub fn new_stream(input: impl Read, input_size: usize) -> Result<Self, ParseException> {
        let xml_parser = XmlPullParser::new_stream(input, input_size)?;

        Ok(Self {
            xml_parser,
            ..Default::default()
        })
    }

    /// The main loop that processes the entire resource
    pub fn parse_markup(&mut self) -> Result<Vec<MarkupElement>, WicketException> {
        let mut stack: Vec<TagPairing> = Vec::new(); // Store indices of open tags
        let mut markup: Vec<MarkupElement> = Vec::new();

        loop {
            // Get the next element from the filter chain.
            let mut tag = match self.get_next_tag()? {
                // Stop if we hit EOF (None)
                None => break,
                Some(MarkupElement::ComponentTag(component_tag)) => component_tag,
                Some(MarkupElement::SpecialTag(special_tag)) => {
                    ComponentTag::from_xml_tag(special_tag.tag)
                }
                Some(MarkupElement::RawMarkup(_)) => unreachable!(),
            };

            let is_wicket_tag = tag.wicket.is_some();
            // The tag is also added if it has been modified by a wicket filter.
            let mut add = is_wicket_tag || tag.is_modified();

            //Check we add the opener for this close
            if !add && tag.tag.is_close() {
                // if let Some(pair_member) = stack.last() {
                if let Some(TagPairing::WicketTag(..)) = stack.last() {
                    // If the matching opener is a wicket tag add the close for inclusion.
                    add = true;
                }
            }

            if add {
                // Add text from the last tag position to the current tag position.
                let text_range = self
                    .xml_parser
                    .get_range_from_position_marker(tag.tag.pos());
                if !text_range.is_empty() {
                    // Check if the previous element in the Vec was also RawMarkup. If so, extend it's
                    // range. Otherwise just add the new raw.
                    if let Some(MarkupElement::RawMarkup(last_raw)) = markup.last_mut() {
                        // If they are contiguous in the source Arc, just extend the range.
                        if last_raw.text_range.end == text_range.start {
                            last_raw.text_range.end = text_range.end;
                        } else {
                            markup.push(MarkupElement::RawMarkup(RawMarkup { text_range }));
                        }
                    } else {
                        markup.push(MarkupElement::RawMarkup(RawMarkup { text_range }));
                    }
                }
                self.xml_parser.set_position_marker_default();
                if tag.tag.is_open() {
                    let current_idx = markup.len();
                    stack.push(TagPairing::WicketTag(current_idx));
                } else if tag.tag.is_close() {
                    if let Some(TagPairing::WicketTag(opener_idx)) = stack.pop() {
                        let current_idx = markup.len();
                        // Adjust the open tag to point to this close tag.
                        if let MarkupElement::ComponentTag(ref mut open_tag) = markup[opener_idx] {
                            if open_tag.tag.name() != tag.tag.name() {
                                // Tags do not match error!
                                let position = tag.tag.pos();
                                let (line, column) = FullyBufferedReader::count_lines_in_str(
                                    &tag.tag.source()[..position],
                                );
                                let close_name = tag.tag.name().into_owned();
                                let open_name = open_tag.tag.name().into_owned();

                                return Err(WicketException::UnmatchedTagName {
                                    close_name,
                                    open_name,
                                    line,
                                    column,
                                    position,
                                });
                            }
                            open_tag.tag.set_open_tag(Some(current_idx));
                        }
                        // Set the close tag's relation.
                        tag.tag.set_close_tag(Some(opener_idx));
                    } else {
                        let position = tag.tag.pos();
                        let (line, column) =
                            FullyBufferedReader::count_lines_in_str(&tag.tag.source()[..position]);
                        return Err(WicketException::NoOpenTag {
                            line,
                            column,
                            position,
                        });
                    }
                }
                tag.tag_id = markup.len() as u16;
                markup.push(MarkupElement::ComponentTag(tag));
            } else {
                //Manage the tag pairing stack for non wicket tags
                if tag.tag.is_open() {
                    stack.push(TagPairing::Raw {
                        tag_type: TagType::Open { closer_index: None },
                        name: tag.tag.name_range,
                    })
                } else if tag.tag.is_close() {
                    match stack.pop() {
                        Some(TagPairing::Raw {
                            tag_type: open_tag_type,
                            name: open_tag_name,
                        }) => match open_tag_type {
                            TagType::Open { .. } => {
                                if open_tag_name.value(self.xml_parser.source()) != tag.tag.name() {
                                    panic!(
                                            "Raw tag type mismatch on opener name: {} vs closer name: {} location:{}",
                                            open_tag_name.value(self.xml_parser.source()),
                                            tag.tag.name(),
                                            self.xml_parser.get_line_and_column_text(),
                                        )
                                }
                            }
                            TagType::Close { .. } => {
                                panic!(
                                        "Raw tag type mismatch: type is not open: {:?} tag name:{} location:{}",
                                        open_tag_type,
                                        open_tag_name.value(self.xml_parser.source()),
                                        self.xml_parser.get_line_and_column_text(),
                                    )
                            }
                            TagType::OpenClose => {
                                panic!(
                                        "Raw tag type mismatch: type is not close: {:?} tag name:{} location:{}",
                                        open_tag_type,
                                        open_tag_name.value(self.xml_parser.source()),
                                        self.xml_parser.get_line_and_column_text(),
                                    )
                            }
                        },
                        Some(TagPairing::WicketTag(indx)) => {
                            panic!("Raw tag type pairing matched on WicketTag({})", indx)
                        }
                        None => panic!("No raw tag type tag pairing match?"),
                    }
                }
            }
        }
        // The stack should be empty.
        if !stack.is_empty() {
            if let Some(MarkupElement::ComponentTag(ct)) = markup
                .iter()
                .find(|x| matches!(x, MarkupElement::ComponentTag(_)))
            {
                let source = ct.tag.source();
                let position = source.len();

                let (line, column) = FullyBufferedReader::count_lines_in_str(&source[..position]);

                return Err(WicketException::NoOpenTag {
                    line,
                    column,
                    position,
                });
            } else {
                unreachable!(
                    "The stack can not contain an element without having a Markup ComponentTag."
                )
            };
        }

        Ok(markup)
    }

    fn get_next_tag(&mut self) -> Result<Option<MarkupElement>, WicketException> {
        // Check internal buffer first (items created by previous filter expansions)
        if let Some(elem) = self.queue.pop_front() {
            return Ok(Some(elem));
        }

        // Pull from the XML parser and run the filter pipeline.
        'outer_loop: loop {
            let mut tag_type: HttpTagType;
            let mut markup_element: Option<MarkupElement>;

            // RootMarkupFilter logic (obtain the xml tag).
            loop {
                tag_type = self.xml_parser.next_iteration()?;

                markup_element = match tag_type {
                    HttpTagType::NotInitialized => return Ok(None),
                    HttpTagType::Body => continue,
                    HttpTagType::Tag => Some(MarkupElement::ComponentTag(
                        ComponentTag::from_xml_tag(self.xml_parser.get_element().unwrap()),
                    )),
                    // SpecialTag processed by HtmlHeaderSectionHandler,
                    // WicketTagIdentifier,OpenCloseTagExpander.
                    _ => Some(MarkupElement::SpecialTag(SpecialTag {
                        tag: self.xml_parser.get_element().unwrap(),
                    })),
                };
                if markup_element.is_some() {
                    break;
                }
            }
            let mut current_item = markup_element.unwrap();

            // Iterate through the filter chain.
            for filter in &mut self.markup_filter_chain {
                match filter.process(current_item)? {
                    FilterResult::Keep(modified) => {
                        current_item = *modified; // Continue to next filter
                    }
                    FilterResult::Drop => {
                        // Stop pipeline, loop back to start to get new XML tag
                        continue 'outer_loop;
                    }
                    FilterResult::Replace(mut list) => {
                        // Complex case: The filter expanded 1 tag into 3.
                        // We take the first one to continue the pipeline,
                        // and queue the rest. These created tags contain modified fields with no
                        // relationship with the source Arc.
                        if !list.is_empty() {
                            current_item = list.remove(0);
                            self.queue.extend(list); // Buffer the rest
                        } else {
                            continue 'outer_loop; // Filter returned empty list (Drop)
                        }
                    }
                }
            }
            return Ok(Some(current_item));
        }
    }
}

#[cfg(test)]
mod test {
    use crate::markup::parser::xml_tag::TagType;

    /// package org.apache.wicket.markup.MarkupParserTest.java;
    use super::*;

    #[test]
    pub fn tag_parsing() {
        // Note: marker is an open tag in the original test. Rust parser
        // faults on an unmatched tag.
        let markup_str = "This is a test <a wicket:id=\"a\" href=\"foo.html\"> \
            <b wicket:id=\"b\">Bold!</b> <img wicket:id=\"img\" \
            width=\"9\" height=\"10\" src=\"foo\"/> <marker wicket:id=\"marker\"/> </a>"
            .to_owned();

        let mut parser = MarkupParser::new(markup_str);
        let markup = parser.parse_markup().unwrap();

        let space = &markup[0];
        assert!(matches!(space, MarkupElement::RawMarkup(rm) if rm.text_range == (0..15)));

        let a_open = &markup[1];
        assert!(matches!(a_open, MarkupElement::ComponentTag(ct) if ct.tag.name() == "a"));
        assert!(
            matches!(a_open, MarkupElement::ComponentTag(ct) if ct.tag.get_attribute_value("href").unwrap()== "foo.html")
        );

        let space = &markup[2];
        assert!(matches!(space, MarkupElement::RawMarkup(rm) if rm.text_range == (48..49)));

        let bold_open = &markup[3];
        assert!(matches!(bold_open, MarkupElement::ComponentTag(ct) if ct.tag.name() == "b"));
        assert!(
            matches!(bold_open, MarkupElement::ComponentTag(ct) if ct.tag.tag_type() == TagType::Open{closer_index:Some(5)}  )
        );

        let bold_text = &markup[4];
        assert!(matches!(bold_text, MarkupElement::RawMarkup(rm) if rm.text_range == (66..71)));

        let bold_close = &markup[5];
        assert!(matches!(bold_close, MarkupElement::ComponentTag(ct) if ct.tag.name() == "b"));
        assert!(
            matches!(bold_close, MarkupElement::ComponentTag(ct) if ct.tag.tag_type() == TagType::Close{opener_index:Some(3)}  )
        );

        let space = &markup[6];
        assert!(matches!(space, MarkupElement::RawMarkup(rm) if rm.text_range == (75..76)));

        let img = &markup[7];
        assert!(matches!(img, MarkupElement::ComponentTag(ct) if ct.tag.name() == "img"));
        assert!(
            matches!(img, MarkupElement::ComponentTag(ct) if ct.tag.get_attribute_int_value("width").unwrap() == 9)
        );
        assert!(
            matches!(img, MarkupElement::ComponentTag(ct) if ct.tag.get_attribute_int_value("height").unwrap() == 10)
        );
        assert!(
            matches!(img, MarkupElement::ComponentTag(ct) if ct.tag.tag_type() == TagType::OpenClose{}  )
        );

        let space = &markup[8];
        assert!(matches!(space, MarkupElement::RawMarkup(rm) if rm.text_range == (130..131)));

        let marker = &markup[9];
        assert!(matches!(marker, MarkupElement::ComponentTag(ct) if ct.tag.name() == "marker"));
        assert!(
            matches!(marker, MarkupElement::ComponentTag(ct) if ct.tag.tag_type() == TagType::OpenClose{}  )
        );

        let _space = &markup[10];

        let a_close = &markup[11];
        assert!(matches!(a_close, MarkupElement::ComponentTag(ct) if ct.tag.name() == "a"));

        assert!(markup.len() == 12);
    }

    #[test]
    pub fn test1() {
        let markup_str = "This is a test <a wicket:id=9> <b>bold</b> <b wicket:id=10></b></a> of the emergency broadcasting system";
        let mut parser = MarkupParser::new(markup_str.to_owned());
        let markup = parser.parse_markup().unwrap();

        let mut text = &markup[0];
        assert!(
            matches!(text, MarkupElement::RawMarkup(rm) if &markup_str[rm.text_range.clone()] == "This is a test ")
        );

        let element = &markup[1];
        assert!(
            matches!(element, MarkupElement::ComponentTag(ct) if ct.tag.get_attribute_int_value("wicket:id") == Some(9))
        );

        text = &markup[2];
        assert!(
            matches!(text, MarkupElement::RawMarkup(rm) if &markup_str[rm.text_range.clone()] == " <b>bold</b> ")
        );
    }

    #[test]
    pub fn wicket_tag() {
        assert!(MarkupParser::new("<span wicket:id=\"test\"/>".to_owned())
            .parse_markup()
            .is_ok());

        assert!(
            MarkupParser::new("<span wicket:id=\"test\">Body</span>".to_owned())
                .parse_markup()
                .is_ok()
        );
        assert!(
            MarkupParser::new("This is a test <span wicket:id=\"test\"/>".to_owned())
                .parse_markup()
                .is_ok()
        );
        assert!(MarkupParser::new(
            "This is a test <span wicket:id=\"test\">Body</span>".to_owned()
        )
        .parse_markup()
        .is_ok());
        assert!(MarkupParser::new(
            "<a wicket:id=\"[autolink]\" href=\"test.html\">Home</a>".to_owned()
        )
        .parse_markup()
        .is_ok());
        assert!(MarkupParser::new("<wicket:body/>".to_owned())
            .parse_markup()
            .is_ok());
        assert!(MarkupParser::new("<wicket:border/>".to_owned())
            .parse_markup()
            .is_ok());
        assert!(MarkupParser::new("<wicket:panel/>".to_owned())
            .parse_markup()
            .is_ok());
        //TODO: Complete <wicket:remove> tag logic tests.
    }

    //TODO: implement the remaining tests from MarkupParserTest.java

    // #[test]
    // TODO: implement StyleAndScriptIdentifier markup filter for the script test.
    pub fn _script() {
        // let  markup = MarkupParser::parse_markup("<html wicket:id=\"test\"><script language=\"JavaScript\">... <x a> ...</script></html>".to_owned());
        let input = "<html wicket:id=\"test\"><script language=\"JavaScript\">... <x a> ...</script></html>";
        let markup = MarkupParser::new(input.to_owned()).parse_markup();
        assert!(markup.as_ref().is_ok_and(|m| m.len() == 5));
        let markup_vec = markup.unwrap();
        assert!(
            matches!(&markup_vec[0], MarkupElement::ComponentTag(ct) if ct.tag.name() == "html")
        );
        assert!(
            matches!(&markup_vec[4], MarkupElement::ComponentTag(ct) if ct.tag.name() == "html")
        );
        assert!(
            matches!(&markup_vec[1], MarkupElement::ComponentTag(ct) if ct.tag.name() == "script")
        );
        assert!(
            matches!(&markup_vec[3], MarkupElement::ComponentTag(ct) if ct.tag.name() == "script")
        );
        // match &markup_vec[2] { MarkupElement::RawMarkup(rmu) => print!("rmu {}", &input[rmu.text_range.clone()]), _ => print!("??"), }
        assert!(
            matches!(&markup_vec[2], MarkupElement::RawMarkup(rmu) if &input[rmu.text_range.clone()] == "... <x a> ..." )
        );
    }
}
