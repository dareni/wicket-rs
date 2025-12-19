//#![allow(unused)]
use std::io::Cursor;
use std::ops::Deref;
use std::{io::Read, rc::Rc};

use wicket_util::wicket::util::collections::io::fully_buffered_reader::{
    FullyBufferedReader, ParseException,
};
use wicket_util::wicket::util::parse::metapattern::parsers::{
    StringVariableAssignmentParser, TagNameParser,
};
use wicket_util::wicket::util::parse::metapattern::{XML_DECL, XML_ENCODING};
use wicket_util::wicket::util::string::strings::unescape_markup;

use super::xml_tag::{TagType, TextSegment, XmlTag};

static STYLE: &str = "style";
static SCRIPT: &str = "script";
static DEFAULT_BUFFER: &str = "";

#[derive(PartialEq)]
pub enum SkipType {
    Style,
    Script,
    Text(String),
    None,
}
impl SkipType {
    pub fn len(&self) -> usize {
        match self {
            Self::Style => STYLE.len(),
            Self::Script => SCRIPT.len(),
            Self::Text(text) => text.len(),
            Self::None => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn get_text(&self) -> &str {
        match self {
            Self::Style => STYLE,
            Self::Script => SCRIPT,
            Self::Text(text) => text.as_str(),
            Self::None => "",
        }
    }
}

pub struct XmlPullParser {
    // Encoding of the xml.
    encoding: String,
    // A XML independent reader which loads the whole source data into memory
    // and which provides convenience methods to access the data.
    input: FullyBufferedReader,
    // Temporary variable which will hold the name of the closing tag
    skip_until_text: SkipType,
    last_type: HttpTagType,
    last_text: Option<Rc<str>>,
    last_tag: Option<XmlTag>,
    doc_type: Option<String>,
}
impl Default for XmlPullParser {
    fn default() -> Self {
        Self {
            encoding: "utf8".to_string(),
            input: FullyBufferedReader::new_from_string(DEFAULT_BUFFER),
            skip_until_text: SkipType::None,
            last_type: HttpTagType::NotInitialized,
            last_text: Option::None as Option<Rc<str>>,
            last_tag: Option::None as Option<XmlTag>,
            doc_type: Option::None as Option<String>,
        }
    }
}

impl XmlPullParser {
    pub fn new(input: String) -> Self {
        Self {
            input: FullyBufferedReader::new_from_string(input),
            ..Default::default()
        }
    }

    pub fn new_stream(mut input: impl Read, input_size: usize) -> Result<Self, ParseException> {
        let mut buffer = Vec::with_capacity(input_size);
        input.read_to_end(&mut buffer)?;
        let encoding_result = determine_encoding(&buffer)?;
        let decoder_opt = encoding_rs::Encoding::for_label(encoding_result.encoding.as_bytes());
        let decoder = decoder_opt.ok_or(ParseException::NoDecoder {
            encoding: encoding_result.encoding.clone(),
        })?;
        // let decoded_cow = decoder.decode(&buffer[encoding_result.bom_len..]).0;
        //
        let decoded_result = decoder.decode(&buffer[encoding_result.bom_len..]);
        let decoded_cow = decoded_result.0;
        let st = decoded_result.1;
        let encoding_name = st.name();

        let buf = Cursor::new(decoded_cow.as_bytes());
        Ok(Self {
            input: FullyBufferedReader::new(buf)?,
            encoding: encoding_name.to_owned(),
            ..Default::default()
        })
    }

    pub fn get_encoding(&self) -> &str {
        self.encoding.as_str()
    }

    pub fn get_doctype(&self) -> Option<&str> {
        self.doc_type.as_deref()
    }

    pub fn get_input_from_position_marker(&self, to_pos: usize) -> &str {
        self.input.get_substring_from_position_marker(Some(to_pos))
    }

    pub fn get_input(&self, from_pos: usize, to_pos: usize) -> Option<&str> {
        self.input.get_substring(from_pos, to_pos)
    }

    fn skip_until(&mut self) -> Result<(), ParseException> {
        let start_index = self.input.get_position();
        let tag_name_len = self.skip_until_text.len();
        let mut pos = self.input.get_position();
        pos = pos.saturating_sub(1);
        let mut last_pos: usize = 0;

        loop {
            pos = self.input.find_str_at("</", pos + 1).ok_or_else(|| {
                ParseException::TagNotClosed {
                    line: self.input.get_line_number(),
                    column: self.input.get_column_number(),
                    position: start_index,
                }
            })?;

            if pos + tag_name_len + 2 >= self.input.size() {
                return Err(ParseException::TagNotClosed {
                    line: self.input.get_line_number(),
                    column: self.input.get_column_number(),
                    position: start_index,
                });
            }
            last_pos += pos + 2;
            let end_tag_text = self
                .input
                .get_substring(last_pos, last_pos + tag_name_len)
                .ok_or_else(|| ParseException::TagEndTextError {
                    line: self.input.get_line_number(),
                    column: self.input.get_column_number(),
                    position: start_index,
                })?
                .to_string();
            if self
                .skip_until_text
                .get_text()
                .eq_ignore_ascii_case(&end_tag_text)
            {
                break;
            }
        }

        self.input.set_position(pos);
        let tmp_last_text = self.input.get_substring(start_index, pos).ok_or_else(|| {
            ParseException::LastTextError {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: start_index,
            }
        })?;
        self.last_text = Some(Rc::<str>::from(tmp_last_text));
        self.last_type = HttpTagType::Body;

        // Check the tag is properly closed
        _ = self
            .input
            .find_char_at('>', last_pos + tag_name_len)
            .ok_or_else(|| ParseException::SkipTagNotClosed {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: start_index,
            })?;
        self.skip_until_text = SkipType::None;
        Ok(())
    }

    pub fn get_line_and_column_text(&self) -> String {
        format!(
            " (line {} , column  {})",
            self.input.get_line_number(),
            self.input.get_column_number()
        )
    }

    pub fn next_iteration(&mut self) -> Result<HttpTagType, ParseException> {
        //Reached end of markup file?
        if self.input.get_position() == self.input.size() {
            return Ok(HttpTagType::NotInitialized);
        }

        if self.skip_until_text != SkipType::None {
            self.skip_until()?;
            return Ok(self.last_type);
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
                return Ok(HttpTagType::Body);
            }
            let text = self
                .input
                .get_substring_from_position_marker(open_bracket_index);
            self.last_text = Some(Rc::from(text));
            match open_bracket_index {
                None => {
                    return Err(ParseException::NoOpenBracketIndexFindingTag(
                        self.input.get_position(),
                    ))
                }
                Some(x) => self.input.set_position(x),
            }
            self.last_type = HttpTagType::Body;
            return Ok(self.last_type);
        }

        // Determine the line number
        let open_bracket_i = match open_bracket_index {
            None => {
                return Err(ParseException::NoOpenBracketIndexSettingLineNo(
                    self.input.get_position(),
                ))
            }
            // Some(x) => {self.input.count_lines_to(x);
            Some(x) => {
                self.input.count_lines_to(x);
                x
            }
        };

        // let open_bracket_i = open_bracket_index.ok_or_else(|| {
        // ParseException::NoOpenBracketIndexGettingTagText(self.input.get_position())
        // })?;

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

        let full_tag_ref = self
            .input
            .get_substring(open_bracket_i, close_bracket_i + 1)
            .unwrap_or_else(|| unreachable!());

        let rc_ref: Rc<str> = Rc::from(full_tag_ref);
        self.last_text = Some(rc_ref.clone());

        // Get the tagtext between open and close brackets
        let full_tag_len = rc_ref.len();
        let mut tag_slice: &str = rc_ref[1..full_tag_len - 1].as_ref();

        if tag_slice.is_empty() {
            return Err(ParseException::EmptyTag {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: self.input.get_position(),
            });
        }

        // Type of the tag, to be determined next
        let tag_type: TagType;

        if tag_slice.ends_with("/") {
            // If the tag ends in '/', it's a "simple" tag like <foo/>
            tag_type = TagType::OpenClose;
            tag_slice = &tag_slice[0..tag_slice.len() - 1];
        } else if tag_slice.starts_with("/") {
            // The tag text starts with a '/', it's a simple close tag
            tag_type = TagType::Close;
            tag_slice = &tag_slice[1..];
        } else {
            // It must be an open tag
            tag_type = TagType::Open;
            // If open tag and starts with "s" like "script" or "style", than ...
            if tag_slice.len() > STYLE.len() && tag_slice[0..1].eq_ignore_ascii_case("s") {
                let lower_case = tag_slice.to_lowercase();
                if lower_case.starts_with(SCRIPT) {
                    // where the type attribute is missing or
                    // where type attribute is text/javascript or importmap or module
                    self.skip_until_text = SkipType::Script;
                } else if lower_case.starts_with(STYLE) {
                    self.skip_until_text = SkipType::Style;
                }
            }
        }

        // Handle special tags like <!-- and <![CDATA ...
        let first_char = tag_slice.chars().next();
        if first_char.is_some_and(|ch| ch == '!' || ch == '?') {
            self.special_tag_handling(tag_slice, open_bracket_i, close_bracket_i)?;

            let text_opt: Option<Rc<str>> = self.last_text.as_ref().map(|v| Rc::from(v.deref()));
            let text = TextSegment::new(
                text_opt,
                open_bracket_i,
                self.input.get_line_number(),
                self.input.get_column_number(),
            );
            self.last_tag = Some(XmlTag::with_text(text, tag_type));
            return Ok(self.last_type);
        }

        let text_opt: Option<Rc<str>> = self.last_text.as_ref().map(|v| Rc::from(v.deref()));
        let text = TextSegment::new(
            text_opt,
            open_bracket_i,
            self.input.get_line_number(),
            self.input.get_column_number(),
        );
        let mut tmp_last_tag = XmlTag::with_text(text, tag_type);

        // Parse the tag text and populate tag attributes
        if self.parse_tag_text(&mut tmp_last_tag, tag_slice)? {
            // Move to position after the tag
            self.input.set_position(close_bracket_i + 1);
            self.last_type = HttpTagType::Tag;
            self.last_tag = Some(tmp_last_tag);
            Ok(self.last_type)
        } else {
            Err(ParseException::MalformedTag {
                line: self.input.get_line_number(),
                column: self.input.get_column_number(),
                position: self.input.get_position(),
            })
        }
    }

    /// Handle special tags like &lt;!-- --&gt; or &lt;![CDATA[..]]&gt; or &lt;?xml&gt;
    fn special_tag_handling(
        &mut self,
        tag_text: &str,
        open_bracket_index: usize,
        mut close_bracket_index: usize,
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
                    .map(Rc::from);

                // Actually it is no longer a comment. It is now
                // up to the browser to select the section appropriate.
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
                    .map(Rc::from);
                self.last_type = HttpTagType::Comment;
                self.input.set_position(pos);
            }
            return Ok(());
        }
        // The closing tag of a conditional comment, e.g.
        // "<!--[if IE]><a href='test.html'>my link</a><![endif]-->
        // and also <!--<![endif]-->"
        if tag_text.eq_ignore_ascii_case("![endif]--") {
            self.last_type = HttpTagType::ConditionalCommentEndif;
            self.input.set_position(close_bracket_index + 1);
            return Ok(());
        }
        // CDATA sections might contain "<" which is not part of an XML tag.
        // Make sure escaped "<" are treated right
        if tag_text.starts_with("![CDATA[") {
            let mut pos1 = open_bracket_index;
            let mut tmp_tag_text: &str;
            loop {
                // Get index of closing tag and advance past the tag
                close_bracket_index = self.find_char('>', pos1).ok_or_else(|| {
                    ParseException::NoCloseBracketIndex {
                        line: self.input.get_line_number(),
                        column: self.input.get_column_number(),
                        position: self.input.get_position(),
                    }
                })?;
                // Get the tagtext between open and close brackets
                tmp_tag_text = self
                    .input
                    .get_substring(open_bracket_index + 1, close_bracket_index)
                    .ok_or_else(|| ParseException::NoSpecialTagText(open_bracket_index + 1))?;

                pos1 = close_bracket_index + 1;
                if tmp_tag_text.ends_with("]]") {
                    break;
                }
            }
            // Move to position after the tag
            self.last_text = Some(Rc::from(tmp_tag_text));
            self.last_type = HttpTagType::Cdata;
            self.input.set_position(close_bracket_index + 1);
            return Ok(());
        }
        if tag_text.starts_with('?') {
            self.last_type = HttpTagType::ProcessingInstruction;
            // Move to position after the tag
            self.input.set_position(close_bracket_index + 1);
            return Ok(());
        }
        if tag_text.starts_with("!DOCTYPE") {
            self.last_type = HttpTagType::Doctype;
            // Get the tagtext between open and close brackets
            self.doc_type = self
                .input
                .get_substring(open_bracket_index + 1, close_bracket_index)
                .map(|s| s.to_owned());
            self.input.set_position(close_bracket_index + 1);
        }
        // Move to position after the tag
        self.last_type = HttpTagType::Special;
        self.input.set_position(close_bracket_index + 1);
        Ok(())
    }

    /// Take the XmlTag.
    pub fn get_element(&mut self) -> Option<XmlTag> {
        self.last_tag.take()
    }

    pub fn get_string(&self) -> Option<&str> {
        self.last_text.as_deref()
    }

    /// Take the next XmlTag.
    pub fn next_tag(&mut self) -> Result<Option<XmlTag>, ParseException> {
        while self.next_iteration()? != HttpTagType::NotInitialized {
            if self.last_type == HttpTagType::Tag {
                return Ok(self.last_tag.take());
            }
        }
        Ok(None)
    }

    /// Find the character 'ch' but ignore any text within ".." and '..'
    ///
    /// Returns the byte index of the first occurrence of 'ch',
    /// or None if not found.
    pub fn find_char(&self, ch: char, start_index: usize) -> Option<usize> {
        let mut quote: Option<char> = None;

        // We use char_indices to get both the byte index and the character,
        // and we skip characters up to the starting byte index.
        for (index, char_at) in self.input.get_substring_from(start_index)?.char_indices() {
            match quote {
                // Inside a quote: Check if we've found the closing quote
                Some(q) if q == char_at => {
                    quote = None;
                }
                // Inside a quote: Continue ignoring
                Some(_) => {}
                // Not inside a quote: Check for opening quote or the target character
                None => {
                    if char_at == '"' || char_at == '\'' {
                        quote = Some(char_at);
                    } else if char_at == ch {
                        return Some(index + start_index); // Found the character!
                    }
                }
            }
        }

        None
    }

    pub fn set_position_marker_default(&mut self) {
        self.input.set_position_marker(self.input.get_position());
    }

    pub fn set_position_marker(&mut self, pos: usize) {
        self.input.set_position_marker(pos);
    }

    pub fn to_string(&self) -> &str {
        self.input.to_string()
    }

    /// Parse the text between tags. For example, "a href=foo.html".
    fn parse_tag_text(&self, tag: &mut XmlTag, tag_text: &str) -> Result<bool, ParseException> {
        let tag_text_length = tag_text.len();

        let tag_name_parser = TagNameParser::new(tag_text);
        // If we match tagname pattern
        if tag_name_parser.is_capture() {
            //Extract the tag from the pattern matcher
            tag.name = Rc::from(tag_name_parser.get_name()?);
            tag.namespace = tag_name_parser.get_namespace().ok().map(|n| n.into());

            // Are we at the end? Then there are no attributes, so we just
            // return the tag
            let mut pos = tag_name_parser.end();
            if pos == tag_text_length {
                return Ok(true);
            }

            loop {
                // Extract attributes
                let attribute_parser = StringVariableAssignmentParser::new(&tag_text[pos..]);

                // Get key and value using attribute pattern
                if !attribute_parser.is_capture() {
                    return Ok(true);
                }

                // In case like <html xmlns:wicket> the value be Error
                let mut value = attribute_parser.get_value().unwrap_or_default();

                // Chop off double quotes or single quotes
                if value.starts_with("\"") || value.starts_with("\'") {
                    value = &value[1..value.len() - 1];
                }

                // Trim trailing whitespace
                value = value.trim();
                // Unescape
                let string_value: String = unescape_markup(value);

                // Get key
                let key = attribute_parser.get_key();

                // Put the attribute in the attributes hash
                match key {
                    Ok(k) => match tag.get_attribute(k) {
                        Some(v) => {
                            return Err(ParseException::AttributeExists {
                                line: self.input.get_line_number(),
                                column: self.input.get_column_number(),
                                position: self.input.get_position(),
                                tag_key: k.to_owned(),
                                tag_value: v.to_owned(),
                            })
                        }
                        None => {
                            tag.put_attribute(k, string_value);
                        }
                    },
                    Err(_) => continue,
                }

                // The input has to match exactly (no left over junk after
                // attributes)
                //
                pos += attribute_parser.end();
                if pos == tag_text_length {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn determine_encoding(xml_head: &str) {
        let _cap_opt = XML_ENCODING.get_regex().captures(xml_head);
    }
}

pub struct EncodingResult {
    pub encoding: String,
    pub bom_len: usize,
}

pub fn determine_encoding(buffer: &[u8]) -> Result<EncodingResult, ParseException> {
    static READ_AHEAD_SIZE: usize = 80;
    let read_ahead = READ_AHEAD_SIZE.min(buffer.len());

    //Assume a string less then 4 bytes is utf8.
    let buf = match buffer.get(0..read_ahead) {
        Some(x) if read_ahead >= 4 => x,
        _ => {
            return Ok(EncodingResult {
                encoding: "utf-8".to_string(),
                bom_len: 0,
            })
        }
    };

    let bom: &[u8] = buf.get(0..4).unwrap();

    let result = match bom {
        [0xFF, 0xFE, 0x00, 0x00] => EncodingResult {
            encoding: "utf-32le".to_string(),
            bom_len: 4,
        },
        [0x00, 0x00, 0xFE, 0xFF] => EncodingResult {
            encoding: "utf-32be".to_string(),
            bom_len: 4,
        },

        // --- 2. UTF-8 BOM (3 Bytes) ---
        [0xEF, 0xBB, 0xBF, ..] => EncodingResult {
            encoding: "utf-8".to_string(),
            bom_len: 3,
        },

        // --- 3. UTF-16 BOMs (Least Specific - 2 Bytes) ---
        // These are checked last for a potential match to ensure UTF-32 was checked first.
        [0xFE, 0xFF, ..] => EncodingResult {
            encoding: "utf-16be".to_string(),
            bom_len: 2,
        },
        [0xFF, 0xFE, ..] => EncodingResult {
            encoding: "utf-16le".to_string(),
            bom_len: 2,
        },

        _ => EncodingResult {
            encoding: "utf-8".to_string(),
            bom_len: 0,
        },
    };

    // check for the <?xml declaration for encoding and cross check with BOM if it exists.
    let xml_start = str::from_utf8(&buf[result.bom_len..])?;
    let xml_decl_opt: Option<&str> = XML_DECL
        .get_regex()
        .captures(xml_start)
        .and_then(|cap| cap.get_match().as_str().into());

    if let Some(xml_decl) = xml_decl_opt {
        let encoding_opt: Option<&str> = XML_ENCODING
            .get_regex()
            .captures(xml_decl)
            .and_then(|cap| cap.get(2).or_else(|| cap.get(3)).map(|mat| mat.as_str()));

        if let Some(encoding) = encoding_opt {
            if result.bom_len == 0 {
                return Ok(EncodingResult {
                    encoding: encoding.to_string(),
                    bom_len: 0,
                });
            } else if encoding.eq_ignore_ascii_case(result.encoding.as_str()) {
                return Ok(result);
            } else {
                return Err(ParseException::XmlEncodingMismatch {
                    bom: result.encoding.to_string(),
                    attribute: encoding.to_string(),
                });
            }
        }
    } else {
        return Err(ParseException::InvalidXmlDeclaration);
    }

    Ok(result)
}

#[derive(PartialEq, Clone, Copy)]
pub enum HttpTagType {
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

#[cfg(test)]
mod test {
    use super::*;

    use crate::wicket::markup::parser::xml_pull_parser::XmlPullParser;

    #[test]
    pub fn basics() {
        let mut parser = XmlPullParser::new("This is text".to_owned());
        let tag = parser.next_tag();
        assert!(tag.is_ok_and(|o| o.is_none()));

        parser = XmlPullParser::new("<tag/>".to_owned());
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open_close());
        assert_eq!("tag", tag.name());
        assert!(tag.namespace().is_none());
        assert!(!tag.has_attributes());

        // extra spaces
        parser = XmlPullParser::new("<tag ></tag >".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("tag", tag.name());
        assert!(tag.namespace().is_none());
        assert!(!tag.has_attributes());

        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("tag", tag.name());
        assert!(tag.namespace().is_none());
        assert!(!tag.has_attributes());

        parser = XmlPullParser::new("<tag> </tag>".to_owned());
        _ = parser.next_tag();
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());

        parser = XmlPullParser::new("xx <tag> yy </tag> zz".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("tag", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("tag", tag.name());

        // XmlPullParser does NOT check that tags get properly closed
        parser = XmlPullParser::new("<tag>".to_owned());
        _ = parser.next_tag();
        assert!(parser.next_tag().unwrap().is_none());

        parser = XmlPullParser::new("<tag> <tag> <tag>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());

        parser = XmlPullParser::new("<ns:tag/>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!("ns", tag.namespace().unwrap());
        assert_eq!("tag", tag.name());
        assert!(tag.is_open_close());

        parser = XmlPullParser::new("<ns:tag></ns:tag>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!("ns", tag.namespace().unwrap());
        assert_eq!("tag", tag.name());
        assert!(tag.is_open());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!("ns", tag.namespace().unwrap());
        assert_eq!("tag", tag.name());
        assert!(tag.is_close());
    }

    #[test]
    pub fn encoding() {
        let mut decl = r#"<?xml version="1.0" encoding="iso-8859-1" ?>"#;
        let mut reader = Cursor::new(decl);
        let mut parser = XmlPullParser::new_stream(reader, decl.len()).unwrap();
        assert_eq!("windows-1252", parser.encoding);
        let tag_opt = parser.next_tag().unwrap();
        assert!(tag_opt.is_none());

        decl = r#"<?xml version="1.0" encoding='iso-8859-1' ?> test test"#;
        reader = Cursor::new(decl);
        parser = XmlPullParser::new_stream(reader, decl.len()).unwrap();
        let tag_opt = parser.next_tag().unwrap();
        assert!(tag_opt.is_none());

        // re-order and move whitespaces
        decl = r#"<?xml encoding='iso-8859-1'version="1.0"?> test test"#;
        reader = Cursor::new(decl);
        parser = XmlPullParser::new_stream(reader, decl.len()).unwrap();
        let tag_opt = parser.next_tag().unwrap();
        assert!(tag_opt.is_none());

        // attribute value must be enclosed by ""
        decl = r#"<?xml encoding=iso-8859-1 ?> test test"#;
        reader = Cursor::new(decl);
        parser = XmlPullParser::new_stream(reader, decl.len()).unwrap();
        assert_eq!("windows-1252", parser.encoding);

        // Invalid encoding
        decl = r#"<?xml encoding='XXX' ?>"#;
        reader = Cursor::new(decl);
        let mut parser_opt = XmlPullParser::new_stream(reader, decl.len());
        assert!(matches!(
            parser_opt,
            Err(ParseException::NoDecoder { encoding: enc }) if enc == "XXX"
        ));

        // no extra characters allowed before <?xml>
        // TODO General: I'd certainly prefer an exception
        decl = r#"xxxx <?xml encoding='iso-8859-1' ?>"#;
        reader = Cursor::new(decl);
        parser_opt = XmlPullParser::new_stream(reader, decl.len());
        assert!(matches!(
            parser_opt,
            Err(ParseException::InvalidXmlDeclaration)
        ));

        //TODO: check for the 3 valid attrbutes eg:
        //<?xml version="1.0" encoding="UTF-8" standalone="yes"?:
    }

    #[test]
    pub fn encoding_string() {
        const ISO_8859_1_XML_BYTES: &[u8] = &[
            // <?xml version="1.0" encoding="ISO-8859-1"?>
            0x3c, 0x3f, 0x78, 0x6d, 0x6c, 0x20, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x3d,
            0x22, 0x31, 0x2e, 0x30, 0x22, 0x20, 0x65, 0x6e, 0x63, 0x6f, 0x64, 0x69, 0x6e, 0x67,
            0x3d, 0x22, 0x49, 0x53, 0x4f, 0x2d, 0x38, 0x38, 0x35, 0x39, 0x2d, 0x31, 0x22, 0x3f,
            0x3e, // padding with space to put the 8859 chars out past READ_AHEAD_SIZE
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            // <name>niño</name>
            0x3c, 0x6e, 0x61, 0x6d, 0x65, 0x3e, 0x6e, 0x69, 0xf1, 0x6f, 0x3c, 0x2f, 0x6e, 0x61,
            0x6d, 0x65, 0x3e,
        ];
        let mut xml_pull_parser =
            XmlPullParser::new_stream(ISO_8859_1_XML_BYTES, ISO_8859_1_XML_BYTES.len()).unwrap();
        let tag = xml_pull_parser.next_tag().unwrap();
        assert_eq!("name".to_owned(), *(tag.unwrap().name).clone());
        //Note: windows-1252 is a super set of iso8859
        assert_eq!("windows-1252", xml_pull_parser.encoding);
    }

    #[test]
    pub fn attributes() {
        let mut parser = XmlPullParser::new("<tag>".to_owned());
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(0, tag.get_attributes().len());
        assert!(!tag.get_attributes().contains_key("attr"));

        parser = XmlPullParser::new("<tag attr='1234'>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(1, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr"));
        assert_eq!("1234", tag.get_attributes().get("attr").unwrap().as_ref());

        parser = XmlPullParser::new("<tag attr=1234>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(1, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr"));
        assert_eq!("1234", tag.get_attributes().get("attr").unwrap().as_ref());

        parser = XmlPullParser::new("<tag attr=1234 >".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(1, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr"));
        assert_eq!("1234", tag.get_attributes().get("attr").unwrap().as_ref());

        parser = XmlPullParser::new("<tag attr-withHypen=1234 >".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(1, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr-withHypen"));
        assert_eq!(
            "1234",
            tag.get_attributes().get("attr-withHypen").unwrap().as_ref()
        );

        parser = XmlPullParser::new(r#"<tag attr="1234">"#.to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(1, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr"));
        assert_eq!("1234", tag.get_attributes().get("attr").unwrap().as_ref());

        parser = XmlPullParser::new("<tag attr='1234' test='23' >".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert_eq!(2, tag.get_attributes().len());
        assert!(tag.get_attributes().contains_key("attr"));
        assert_eq!("1234", tag.get_attributes().get("attr").unwrap().as_ref());
        assert!(tag.get_attributes().contains_key("test"));
        assert_eq!("23", tag.get_attributes().get("test").unwrap().as_ref());

        parser = XmlPullParser::new("<tag attr='1234' attr='23' >".to_owned());
        assert!(matches!(
            parser.next_tag(),
            Err(ParseException::AttributeExists { .. })
        ));
    }

    #[test]
    pub fn comments() {
        let mut parser = XmlPullParser::new("<!-- test --><tag>".to_owned());
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert_eq!("tag", tag.name());

        let mut parser = XmlPullParser::new(
            "<!-- test --><tag> aaa <!-- test 1 --> bbb <tag> <!-- test --> </tag>".to_owned(),
        );
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("tag", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("tag", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("tag", tag.name());
        assert!(parser.next_tag().unwrap().is_none());
    }

    #[test]
    pub fn script() {
        let mut parser = XmlPullParser::new(
            "<html><script language=\"JavaScript\">... <x a> ...</script></html>".to_owned(),
        );
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("html", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("script", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("script", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("html", tag.name());
    }

    #[test]
    pub fn skip_script_tag() {
        let mut parser = XmlPullParser::new(
"<html><script type=\"module\">all I need is a < char to break parser </script><body></body></html>".to_owned()
        );
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("html", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("script", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("script", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("body", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("body", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("html", tag.name());
    }

    #[test]
    pub fn conditional_comments() {
        let mut parser = XmlPullParser::new(
            "<!--[if IE]><a href='test.html'>my link</a><![endif]-->".to_owned(),
        );
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("a", tag.name());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_close());
        assert_eq!("a", tag.name());
        assert!(parser.next_tag().unwrap().is_none());
    }

    #[test]
    pub fn conditional_comments2() {
        let mut parser = XmlPullParser::new(
            "<!--[if IE]><a href='test.html'>my link</a><![endif]-->".to_owned(),
        );
        let mut tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::ConditionalComment));
        tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::Tag));
        assert!(parser.get_element().unwrap().is_open());
        tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::Body));
        tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::Tag));
        let tag = parser.get_element().unwrap();
        assert_eq!("a", tag.name());
        assert!(tag.is_close());
        tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::ConditionalCommentEndif));
        tag_type = parser.next_iteration().unwrap();
        assert!(matches!(tag_type, HttpTagType::NotInitialized));
    }

    #[test]
    pub fn names() {
        let mut parser = XmlPullParser::new("<filter-mapping>".to_owned());
        let mut tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("filter-mapping", tag.name());

        parser = XmlPullParser::new("<filter.mapping>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("filter.mapping", tag.name());

        parser = XmlPullParser::new("<filter_mapping>".to_owned());
        tag = parser.next_tag().unwrap().unwrap();
        assert!(tag.is_open());
        assert_eq!("filter_mapping", tag.name());
    }

    #[test]
    pub fn doctype() {
        let mut parser = XmlPullParser::new("<!DOCTYPE html>".to_owned());
        let _tag_type = parser.next_iteration().unwrap();
        assert!(matches!(HttpTagType::Doctype, _tag_type));
        assert_eq!("!DOCTYPE html", parser.get_doctype().unwrap());
    }

}
