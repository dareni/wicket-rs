#![allow(unused)]

use std::rc::Rc;
use std::{cmp::Ordering, ops::Deref};
use thiserror::Error;

use wicket_util::wicket::util::collections::io::fully_buffered_reader::{
    FullyBufferedReader, ParseException,
};

use super::xml_tag::{TagType, TextSegment, XmlTag};

static STYLE: &str = "style";
static SCRIPT: &str = "script";

#[derive(PartialEq)]
enum SkipType {
    Style,
    Script,
    None,
}

impl SkipType {
    fn value(&self) -> &str {
        match *self {
            Self::Style => "style",
            Self::Script => "script",
            Self::None => "",
        }
    }
}

pub fn parse() {
    println!("parsing");
}

struct XmlPullParser {
    // Encoding of the xml.
    encoding: String,

    // A XML independent reader which loads the whole source data into memory
    // and which provides convenience methods to access the data.
    input: FullyBufferedReader,
    //
    // Temporary variable which will hold the name of the closing tag
    skip_until_text: SkipType,
    last_type: HttpTagType,
    last_text: Option<Rc<str>>,
    last_tag: Option<XmlTag>,
}

impl XmlPullParser {
    fn new(input: String) -> Self {
        Self {
            encoding: "utf8".to_string(),
            input: FullyBufferedReader::new_from_string(input),
            skip_until_text: SkipType::None,
            last_type: HttpTagType::NotInitialized,
            last_text: Option::None as Option<Rc<str>>,
            last_tag: Option::None as Option<XmlTag>,
        }
    }

    fn next(&mut self) -> Result<&HttpTagType, ParseException> {
        //Reached end of markup file?
        if self.input.get_position() == self.input.size() {
            return Ok(&HttpTagType::NotInitialized);
        }

        if self.skip_until_text != SkipType::None {
            //TODO self.skip_until();
            return Ok(&self.last_type);
        }

        //Any more tags in the markup?
        let open_bracket_index = self.input.find_char('<');

        //Tag or Body?
        if self.input.char_at(self.input.get_position()) != '<' {
            //It's a BODY
            if open_bracket_index.is_none() {
                //There is no next matching tag.
                let text = self.input.get_substring_from_position_marker(Option::None);
                if text.trim().is_empty() {
                    self.last_text = None;
                } else {
                    self.last_text = Some(Rc::from(text));
                }
                self.input.set_position(self.input.size());
                self.last_type = HttpTagType::Body;
            }
            let text = self
                .input
                .get_substring_from_position_marker(open_bracket_index);
            self.last_text = Some(Rc::from(text.to_string()));
            match open_bracket_index {
                None => {
                    return Err(ParseException::NoOpenBracketIndexFindingTag(
                        self.input.get_position(),
                    ))
                }
                Some(x) => self.input.set_position(x),
            }
            self.last_type = HttpTagType::Body;
            return Ok(&self.last_type);
        }

        // Determine the line number
        match open_bracket_index {
            None => {
                return Err(ParseException::NoOpenBracketIndexSettingLineNo(
                    self.input.get_position(),
                ))
            }
            Some(x) => self.input.count_lines_to(x),
        }

        let open_bracket_i = open_bracket_index.ok_or_else(|| {
            ParseException::NoOpenBracketIndexGettingTagText(self.input.get_position())
        })?;

        // Get index of closing tag and advance past the tag
        let mut close_bracket_index: Option<usize> = None;
        if open_bracket_i < self.input.size() - 1 {
            let next_char = self.input.char_at(open_bracket_i + 1);
            match next_char {
                '!' | '?' => close_bracket_index = self.input.find_char_at('>', open_bracket_i),
                _ => {
                    close_bracket_index =
                        self.input.find_out_of_quotes('>', open_bracket_i, None)?
                }
            }
        }
        let close_bracket_i =
            close_bracket_index.ok_or_else(|| ParseException::NoCloseBracketIndex {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: self.input.get_position(),
            })?;

        // Get the complete tag text
        self.last_text = self
            .input
            .get_substring(open_bracket_i, close_bracket_i + 1)
            .map(Rc::from);

        // Get the tagtext between open and close brackets
        let mut tag_text;
        if let Some(last_t) = &self.last_text {
            tag_text = last_t[0..last_t.len() - 1].to_string();
            if tag_text.is_empty() {
                return Err(ParseException::EmptyTag {
                    line: self.input.get_line_number(),
                    column: self.input.get_column_number(),
                    position: self.input.get_position(),
                });
            }
        } else {
            unreachable!()
        }

        // Type of the tag, to be determined next
        let tag_type: TagType;

        if tag_text.ends_with("/") {
            // If the tag ends in '/', it's a "simple" tag like <foo/>
            tag_type = TagType::OpenClose;
            tag_text = tag_text[0..tag_text.len()].to_string();
        } else if tag_text.starts_with("/") {
            // The tag text starts with a '/', it's a simple close tag
            tag_type = TagType::Close;
            tag_text = tag_text[1..].to_string();
        } else {
            // It must be an open tag
            tag_type = TagType::Open;
            // If open tag and starts with "s" like "script" or "style", than ...
            if tag_text.len() > STYLE.len() && tag_text.starts_with('s')
                || tag_text.starts_with('S')
            {
                let lower_case = tag_text.to_lowercase();
                if lower_case.starts_with(SCRIPT) {
                    // where the type attribute is missing or
                    // where type attribute is text/javascript or importmap or module
                    self.skip_until_text = SkipType::Script;
                } else if (lower_case.starts_with(STYLE)) {
                    self.skip_until_text = SkipType::Style;
                }
            }
        }

        // Handle special tags like <!-- and <![CDATA ...
        let first_char = tag_text.chars().next();
        if first_char.is_some_and(|ch| ch == '!' || ch == '?') {
            self.special_tag_handling(tag_text.as_str(), open_bracket_i, close_bracket_i);
            self.input.count_lines_to(open_bracket_i);

            let text_opt: Option<Rc<str>> = self.last_text.as_ref().map(|v| Rc::from(v.deref()));

            let text = TextSegment::new(
                text_opt,
                open_bracket_i,
                self.input.get_line_number(),
                self.input.get_column_number(),
            );
            self.last_tag = Some(XmlTag::with_text(text, tag_type));
            return Ok(&self.last_type);
        }

        let text_opt: Option<Rc<str>> = self.last_text.as_ref().map(|v| Rc::from(v.deref()));
        let text = TextSegment::new(
            text_opt,
            open_bracket_i,
            self.input.get_line_number(),
            self.input.get_column_number(),
        );
        self.last_tag = Some(XmlTag::with_text(text, tag_type));

        // Parse the tag text and populate tag attributes
        if self.parse_tag_text(&tag_text) {
            // Move to position after the tag
            self.input.set_position(close_bracket_i + 1);
            self.last_type = HttpTagType::Tag;
            return Ok(&self.last_type);
        } else {
            return Err(ParseException::MalformedTag {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: self.input.get_position(),
            });
        }

        Ok(&HttpTagType::Tag)
    }

    /// Handle special tags like &lt;!-- --&gt; or &lt;![CDATA[..]]&gt; or &lt;?xml&gt;
    fn special_tag_handling(
        &mut self,
        tag_text: &str,
        open_bracket_index: usize,
        close_bracket_index: usize,
    ) -> Result<(), ParseException> {
        // Handle comments
        if tag_text.starts_with("!--") {
            // downlevel-revealed conditional comments e.g.: <!--[if (gt IE9)|!(IE)]><!-->
            if tag_text.contains("![endif]--") {
                self.last_type = HttpTagType::ConditionalCommentEndif;
                // Move to position after the tag
                self.input.set_position(close_bracket_index + 1);
                return Ok(());
            }
            // Conditional comment? E.g.
            // "<!--[if IE]><a href='test.html'>my link</a><![endif]-->"
            if tag_text.starts_with("!--[if ") && tag_text.ends_with("]") {
                let pos_option = self.input.find_str_at("]-->", open_bracket_index + 1);
                let mut pos = pos_option.ok_or_else(|| ParseException::UnclosedComment {
                    line: self.input.get_line_number(),
                    column: self.input.get_column_number(),
                    position: self.input.get_position(),
                })?;
                pos += 4;
                self.last_text = self
                    .input
                    .get_substring(open_bracket_index, pos)
                    .map(|s| Rc::from(s.to_string().into_boxed_str()));
                self.input.set_position(close_bracket_index + 1);
                self.last_type = HttpTagType::ConditionalComment;
            } else {
                // Normal comment section.
                // Skip ahead to "-->". Note that you can not simply test for
                // tagText.endsWith("--") as the comment might contain a '>'
                // inside.
                let mut pos = self
                    .input
                    .find_str_at("-->", open_bracket_index + 1)
                    .ok_or_else(|| ParseException::UnclosedComment {
                        line: self.input.get_line_number(),
                        column: self.input.get_column_number(),
                        position: self.input.get_position(),
                    })?;
                pos += 3;
                self.last_text = self
                    .input
                    .get_substring(open_bracket_index, pos)
                    .map(|s| s.to_string().into_boxed_str().into());
                self.last_type = HttpTagType::Comment;
                self.input.set_position(pos);
            }
        }
        Ok(())
    }

    fn parse_tag_text(&self, tag_text: &str) -> bool {
        // Get the length of the tagtext
        let tag_text_length = tag_text.len();

        // If we match tagname pattern

        // TODO:
        true
    }
}

trait IXmlPullParser {
    fn get_encoding(&self) -> &str;
}

enum HttpTagType {
    // next() must be called at least once for the Type to be valid
    NotInitialized,

    // <name>
    Tag,

    // Tag body in between two tags
    Body,

    // !--
    Comment,

    // <!--[if ] ... -->
    ConditionalComment,

    // <![endif]-->
    ConditionalCommentEndif,

    // <![CDATA[ .. ]]>
    Cdata,

    // <?...>
    ProcessingInstruction,

    // <!DOCTYPE ...>
    Doctype,

    //all other tags which look like <!.. >
    Special,
}
