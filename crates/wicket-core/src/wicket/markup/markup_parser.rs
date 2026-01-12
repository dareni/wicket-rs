use std::borrow::Cow;

use once_cell::sync::Lazy;

use crate::wicket::markup::parser::filter::RootMarkupFilter;
pub(crate) use crate::wicket::{
    markup::{
        parser::{xml_pull_parser::XmlPullParser, MarkupFilter},
        Markup,
    },
    settings::MarkupSettings,
};
use wicket_util::static_pattern;
use wicket_util::wicket::util::parse::metapattern::{Pattern, RegexFlags};

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
    pub markup_filter_chain: Box<dyn MarkupFilter>,
    // The maarkup created from the input markup file.
    pub markup: Markup,
    pub markup_settings: MarkupSettings,
    pub filters: Vec<Box<dyn MarkupFilter>>,
}

impl Default for MarkupParser {
    fn default() -> Self {
        Self {
            xml_parser: Default::default(),
            markup_filter_chain: Box::new(RootMarkupFilter::new(XmlPullParser::new("".into()))),
            markup: Markup::new(),
            markup_settings: MarkupSettings::default(),
            filters: Default::default(),
        }
    }
}

impl MarkupParser {
    pub fn new(input: String) -> Self {
        Self {
            xml_parser: XmlPullParser::new(input),
            ..Default::default()
        }
    }
}
