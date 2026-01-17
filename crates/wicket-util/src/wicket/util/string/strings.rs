use std::borrow::Cow;

use html_escape::decode_html_entities;

pub fn unescape_markup(markup: &str) -> Cow<'_, str> {
    decode_html_entities(markup)
}
