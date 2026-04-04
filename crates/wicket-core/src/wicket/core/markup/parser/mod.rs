use wicket_util::wicket::util::collections::io::fully_buffered_reader::ParseException;

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
    )]
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
