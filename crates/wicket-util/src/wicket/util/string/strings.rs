use html_escape::decode_html_entities;

pub fn unescape_markup(markup: &str) -> String {
    let ret = decode_html_entities(markup);
    ret.into_owned()
}
