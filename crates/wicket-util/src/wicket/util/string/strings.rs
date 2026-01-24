use std::borrow::Cow;

use html_escape::decode_html_entities;
use html_escape::encode_double_quoted_attribute;

pub fn unescape_markup(markup: &str) -> Cow<'_, str> {
    decode_html_entities(markup)
}

pub fn escape_markup(markup: &str) -> Cow<'_, str> {
    encode_double_quoted_attribute(markup)
}
