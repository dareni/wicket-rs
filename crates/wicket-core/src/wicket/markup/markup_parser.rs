use crate::wicket::markup::markup_element::RawMarkup;
use std::borrow::Cow;
use std::collections::VecDeque;

use once_cell::sync::Lazy;

use crate::wicket::markup::{
    markup_element::{ComponentTag, MarkupElement, SpecialTag},
    parser::{xml_pull_parser::HttpTagType, FilterResult},
};
pub(crate) use crate::wicket::{
    markup::{
        parser::{xml_pull_parser::XmlPullParser, MarkupFilter},
        Markup,
    },
    settings::MarkupSettings,
};
use wicket_util::wicket::util::parse::metapattern::{Pattern, RegexFlags};
use wicket_util::{
    static_pattern, wicket::util::collections::io::fully_buffered_reader::ParseException,
};

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
    pub filters: Vec<Box<dyn MarkupFilter>>,
    pub queue: VecDeque<MarkupElement>,
}

impl Default for MarkupParser {
    fn default() -> Self {
        let markup_filter_chain: Vec<Box<dyn MarkupFilter>> = Vec::with_capacity(10);
        Self {
            xml_parser: Default::default(),
            markup_filter_chain,
            markup: Markup::new(),
            markup_settings: MarkupSettings::default(),
            filters: Default::default(),
            queue: Default::default(),
        }
    }
}

impl MarkupParser {
    //TODO: Add postProcess() and filter chain configuration.

    pub fn new(input: String) -> Self {
        Self {
            xml_parser: XmlPullParser::new(input),
            ..Default::default()
        }
    }

    /// The main loop that processes the entire resource
    pub fn parse_markup(&mut self) -> Result<Vec<MarkupElement>, ParseException> {
        let mut markup: Vec<MarkupElement> = Vec::new();

        loop {
            // Get the next element from the filter chain.
            let tag = match self.get_next_tag()? {
                // Stop if we hit EOF (None)
                None => break,
                Some(MarkupElement::ComponentTag(component_tag)) => component_tag,

                Some(MarkupElement::SpecialTag(special_tag)) => {
                    ComponentTag::from_xml_tag(special_tag.tag)
                }
                Some(MarkupElement::RawMarkup(_)) => unreachable!(),
            };
            //Add the tag if it contains a wicket id.
            let mut add = !tag.get_id().is_empty();

            // Add the tag when it is a close for a wicket tag.
            if !add && tag.tag.is_close() {
                add = tag.get_open_tag().is_some()
                    && !&tag.get_open_tag().as_ref().unwrap().get_id().is_empty();
            }

            // The tag is also added if it has been modified by a wicket filter.
            if add || tag.is_modified() {
                // Add text from the last tag position to the current tag position.
                let text_range = self
                    .xml_parser
                    .get_range_from_position_marker(tag.tag.pos());
                if !text_range.is_empty() {
                    // Check if the previous element in the Vec was also RawMarkup and extend it's
                    // range if it is to include the text from this next tag.
                    if let Some(MarkupElement::RawMarkup(last_raw)) = markup.last_mut() {
                        // Optimization: If they are contiguous in the source Arc, just extend the range
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
                markup.push(MarkupElement::ComponentTag(tag));
            }
        }

        Ok(markup)
    }

    pub fn get_next_tag(&mut self) -> Result<Option<MarkupElement>, ParseException> {
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
            for filter in &mut self.filters {
                match filter.process(&mut current_item) {
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
