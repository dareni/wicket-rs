use std::io::{self, Read};

/// This is not a reader like e.g. FileReader. It rather reads the whole data until the end from a
/// source reader into memory and provides convenient methods for navigation and searching.
pub struct FullyBufferedReader {
    /// All the chars from the resource.
    input: String,

    /// Current position in the input.
    input_position: usize,

    /// Current line number.
    line_number: usize,

    /// Current column number.
    column_number: usize,

    /// Last place we counted lines from.
    last_line_count_index: usize,

    /// A variable to remember a certain position in the markup
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
#[derive(Debug)]
pub struct ParseException {
    message: String,
    position: usize,
}

impl std::fmt::Display for ParseException {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "ParseException: {} at position {}",
            self.message, self.position
        )
    }
}

impl std::error::Error for ParseException {}

impl FullyBufferedReader {
    /// Read all the data from the `reader` into memory.
    pub fn new(mut reader: impl Read) -> io::Result<Self> {
        let mut input = String::new();
        reader.read_to_string(&mut input)?;
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
    /// Set `toPos` as the index of the last character non inclusive.
    /// If `toPos`` > 0, then get all data from the position marker to the end.
    /// If `toPos`` is less than the position marker then return an empty string.
    /// A string of raw markup in between these to two positions is returned.
    pub fn get_substring_from_position_marker(&self, to_pos: isize) -> &str {
        if to_pos < 0 {
            &self.input[self.position_marker..]
        } else if (to_pos as usize) < self.position_marker {
            ""
        } else {
            &self.input[self.position_marker..to_pos as usize]
        }
    }

    /// Get the characters from in between both positions including
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
    // @param end End index.
    pub fn count_lines_to(&mut self, end: usize) {
        let input_slice = &self.input[self.last_line_count_index..end];
        let sl = input_slice.bytes();
        sl.for_each(|ch| {
            match ch {
                b'\n' => {
                    self.column_number = 1;
                    self.line_number += 1;
                }
                b'\r' => {
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

    /// Find the string starting ath the current input position.
    pub fn find_str(&self, strg: &str) -> Option<usize> {
        self.input[self.input_position..]
            .find(strg)
            .map(|i| i + self.input_position)
    }

    // Find the string starting at the position provided.
    // @param strg The string to search for.
    // @param start_pos The index to start the search.
    // @return -1 if not found.
    pub fn find_str_at(&self, strg: &str, start_pos: usize) -> Option<usize> {
        self.input[start_pos..].find(strg).map(|i| i + start_pos)
    }

    // Find a char starting at the position provided. The char must not be
    // inside a quoted string.  (single or double)
    // @param ch The char to search for.
    // @param startPos The index to start at.
    // @param quotationChar The current quotation char. Must be ' or ",
    // otherwise will be ignored.
    // @return -1 if not found
    pub fn find_out_of_quotes(
        &self,
        ch: char,
        start_pos: usize,
        quotation_char: Option<char>,
    ) -> Result<Option<usize>, ParseException> {
        let mut current_quotation_char = quotation_char;
        let mut i = start_pos;
        while i < self.input.len() {
            let current_char = self.input.as_bytes()[i] as char;

            if current_quotation_char.is_none() {
                if current_char == '\'' || current_char == '\"' {
                    current_quotation_char = Some(current_char);
                }
            } else {
                let previous_char = if i > 0 {
                    self.input.as_bytes()[i - 1] as char
                } else {
                    '\0'
                };
                if current_char == current_quotation_char.unwrap() && previous_char != '\\' {
                    current_quotation_char = None;
                }
            }

            if current_char == ch && current_quotation_char.is_none() {
                return Ok(Some(i));
            }
            i += 1;
        }

        if current_quotation_char.is_some() {
            return Err(ParseException {
                message: "Opening/closing quote not found".to_string(),
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
    pub fn char_at(self, pos: usize) -> char {
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
        let fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let position = fully_buffered_reader
            .find_out_of_quotes('>', 0, None)
            .unwrap();
        println!("> is at {}", position.unwrap());
        assert_eq!('>', test_tag.as_bytes()[position.unwrap()] as char);
    }
}
