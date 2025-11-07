use std::io::{self, Read};
use thiserror::Error;

/// This is not a reader like e.g. FileReader. It rather reads the whole data until the end from a
/// source reader into memory and provides convenient methods for navigation and searching.
pub struct FullyBufferedReader {
    /// All the chars from the resource.
    input: String,

    /// Current position in the input.
    input_position: usize,

    /// Current line number.
    line_number: usize,

    /// Current column number (chars not bytes).
    column_number: usize,

    /// Last place we counted lines from.
    last_line_count_index: usize,

    /// A variable to remember a certain byte index position in the markup
    position_marker: usize,
}

impl Default for FullyBufferedReader {
    fn default() -> Self {
        Self {
            input: "".to_string(),
            input_position: 0,
            line_number: 1,
            column_number: 1,
            last_line_count_index: 0,
            position_marker: 0,
        }
    }
}

// Custom error for parsing exceptions
#[derive(Debug, Error)]
pub enum ParseException {
    #[error(
        "ParseException: Opening/closing quote not found for quote at (line \
        {line_number}, column {column_number}) position {position}"
    )]
    Find {
        line_number: usize,
        column_number: usize,
        position: usize,
    },
    #[error("ParseException: IO error caused by {cause}.")]
    IO {
        #[source]
        cause: io::Error,
    },
    #[error("Invalid character found at position {0}")]
    InvalidChar(usize),
}

impl FullyBufferedReader {
    /// Read all the data from the `reader` into memory.
    pub fn new(mut reader: impl Read) -> Result<Self, ParseException> {
        let mut input = String::new();
        reader
            .read_to_string(&mut input)
            .map_err(|e| ParseException::IO { cause: e })?;
        Ok(Self {
            input,
            ..Self::default()
        })
    }

    /// Construct a `FullyBufferedReader` from the `input` string.
    pub fn new_from_string(input: String) -> Self {
        Self {
            input,
            ..Self::default()
        }
    }

    /// Get the characters from the internal position marker to `toPos`.
    /// Set `toPos` as the byte index of the last utf8 character non inclusive.
    /// If `toPos`` > 0, then get all data from the position marker to the end.
    /// If `toPos`` is less than the position marker then return an empty string.
    /// A string of raw markup in between these to two positions is returned.
    pub fn get_substring_from_position_marker(&self, to_pos: Option<usize>) -> &str {
        match to_pos {
            None => &self.input[self.position_marker..],
            Some(x) if x < self.position_marker => "",
            Some(x) => &self.input[self.position_marker..x],
        }
    }

    /// Get the utf8 characters from in between both positions(byte indices) including
    /// the char at fromPos, excluding the char at toPos.
    pub fn get_substring(&self, from_pos: usize, to_pos: usize) -> Option<&str> {
        self.input.get(from_pos..to_pos)
    }

    //Get the current input buffer position.
    pub fn get_position(&self) -> usize {
        self.input_position
    }

    // Store the current position in markup.
    pub fn set_position_marker(&mut self, pos: usize) {
        self.position_marker = pos;
    }

    // @return The markup to be parsed.
    pub fn to_string(&self) -> &str {
        &self.input
    }

    // Counts lines starting where we last left off up to the index provided.
    // `end` must be the byte index to a utf8 char.
    pub fn count_lines_to(&mut self, end: usize) {
        let input_slice = &self.input[self.last_line_count_index..end];
        let sl = input_slice.chars();
        sl.for_each(|ch| {
            match ch {
                '\n' => {
                    self.column_number = 1;
                    self.line_number += 1;
                }
                '\r' => {
                    // Do nothing.
                }
                _ => {
                    self.column_number += 1;
                }
            }
        });
        self.last_line_count_index = end;
    }

    /// Find a char starting at the current input position.
    pub fn find_char(&self, ch: char) -> Option<usize> {
        self.input[self.input_position..]
            .find(ch)
            .map(|x| x + self.input_position)
    }

    /// Find a char starting at the position provided.
    pub fn find_char_at(&self, ch: char, start_pos: usize) -> Option<usize> {
        self.input[start_pos..].find(ch).map(|i| i + start_pos)
    }

    /// Find the string starting at the current input position.
    pub fn find_str(&self, strg: &str) -> Option<usize> {
        self.input[self.input_position..]
            .find(strg)
            .map(|i| i + self.input_position)
    }

    // Find the string starting at the position provided.
    pub fn find_str_at(&self, strg: &str, start_pos: usize) -> Option<usize> {
        self.input[start_pos..].find(strg).map(|i| i + start_pos)
    }

    // Find a char starting at the position provided. The char must not be
    // inside a quoted string.  (single or double)
    // @param quotationChar The current quotation char. Must be ' or ",
    // otherwise will be ignored.
    pub fn find_out_of_quotes(
        &mut self,
        ch: char,
        start_pos: usize,
        quotation_char: Option<char>,
    ) -> Result<Option<usize>, ParseException> {
        let mut current_quotation_char = quotation_char;
        let mut i = start_pos;
        let mut previous_char: Option<char> = None;
        while i < self.input.len() {
            let current_char = self.input[i..]
                .chars()
                .next()
                .ok_or(ParseException::InvalidChar(i))?;

            if current_quotation_char.is_none() {
                if current_char == '\'' || current_char == '\"' {
                    current_quotation_char = Some(current_char);
                    self.count_lines_to(start_pos + i);
                }
            } else if current_quotation_char.is_some_and(|q_char| q_char == current_char)
                && previous_char.is_some_and(|pc| pc != '\\')
            {
                current_quotation_char = None;
            }

            if current_char == ch && current_quotation_char.is_none() {
                return Ok(Some(i));
            }
            previous_char = Some(current_char);
            i += current_char.len_utf8();
        }

        if current_quotation_char.is_some() {
            return Err(ParseException::Find {
                line_number: self.get_line_number(),
                column_number: self.get_column_number(),
                position: start_pos,
            });
        }

        Ok(None)
    }

    /// Position the reader at the index provided. Could be anywhere within
    /// the data.
    pub fn set_position(&mut self, pos: usize) {
        self.input_position = pos;
    }

    /// Get the column number. Note: The column number depends on you calling
    /// countLinesTo(pos). It is not necessarily the column number matching the
    /// current position in the stream.
    pub fn get_column_number(&self) -> usize {
        self.column_number
    }

    /// Get the line number. Note: The line number depends on you calling
    /// countLinesTo(pos). It is not necessarily the line number matching the
    /// current position in the stream.
    pub fn get_line_number(&self) -> usize {
        self.line_number
    }

    // Get the number of character read from the source resource. The whole
    // content, not just until the current position.
    pub fn size(&self) -> usize {
        self.input.len()
    }

    // Get the character at the position provided.
    // @param pos The position.
    // @return char at position.
    pub fn char_at(&self, pos: usize) -> char {
        self.input.as_bytes()[pos] as char
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nested_quotes() {
        // test_tag is <a href='b \'" > a' theAtr="at'r'\"r">
        let test_tag = "<a href='b \\'\" > a' theAtr=\"at'r'\\\"r\">";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let position = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap();
        assert_eq!('>', test_tag.as_bytes()[position.unwrap()] as char);
        assert_eq!(test_tag.len(), position.unwrap() + 1);
    }

    #[test]
    fn quoted_esclamation_quotation_mark() {
        let test_tag = "<a href='b \" >!! a<??!!' theAtr=\">\">";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let position = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap();
        assert_eq!('>', test_tag.as_bytes()[position.unwrap()] as char);
        assert_eq!(test_tag.len(), position.unwrap() + 1)
    }
    #[test]
    fn missing_closing_quote() {
        let test_tag = "<a href='blabla>";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let error = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "ParseException: Opening/closing quote not found for quote at (line 1, column 9) position 0"
        );
    }
    #[test]
    fn missing_opening_quote() {
        let test_tag = "<a href=blabla'>";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let error = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "ParseException: Opening/closing quote not found for quote at (line 1, column 15) position 0"
        );
    }
    #[test]
    fn missing_closing_double_quote() {
        let test_tag = "<a href=\"blabla>";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let error = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "ParseException: Opening/closing quote not found for quote at (line 1, column 9) position 0"
        );
    }
    #[test]
    fn missing_opening_double_quote() {
        let test_tag = "<a href=blabla\"a>";
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let error = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "ParseException: Opening/closing quote not found for quote at (line 1, column 15) position 0"
        );
    }
}
