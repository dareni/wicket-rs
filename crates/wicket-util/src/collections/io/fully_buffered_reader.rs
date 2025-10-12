use std::io::{BufReader, Error, Read};

pub struct FullyBufferedReader {
    // All the chars from the resource.
    input: String,

    // Position in parse.
    input_position: usize,

    // Current line number.
    line_number: usize,

    // Current column number.
    column_number: usize,

    // Last origin of line count.
    last_line_count_index: usize,

    // Input markup position counter.
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

impl FullyBufferedReader {
    /// Construct a `FullyBufferedReader`.
    /// Read all the data from the `reader` into memory.
    /// Set `reader` as the source to load the data from.
    pub fn new<T>(mut reader: BufReader<T>) -> Result<FullyBufferedReader, Error>
    where
        T: Read,
    {
        let mut buf: Vec<u8> = Vec::new();
        let _size: usize = reader.read_to_end(&mut buf)?;
        //TODO throw an exception for zero size.

        let input = match String::from_utf8(buf) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        Ok(FullyBufferedReader {
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
    pub fn get_substring_from_position_marker(&self, to_pos: i32) -> Option<&str> {
        let pos;
        if to_pos < 0 {
            pos = self.input.len();
        } else if to_pos < self.position_marker as i32 {
            return Some("");
        } else {
            pos = to_pos as usize;
        }
        self.input.get(self.position_marker..pos)
    }

    // Get the characters from in between both positions including
    // the char at fromPos, excluding the char at toPos.
    // @param from_pos first index.
    // @param to_pos second index.
    // @return the string (raw markup) in between both positions.
    pub fn get_substring(&self, from_pos: usize, to_pos: usize) -> Option<&str> {
        self.input.get(from_pos..to_pos)
    }

    //Get the current input buffer position.
    pub fn get_position(&self) -> usize {
        self.input_position
    }

    // Store the current position in markup.
    // @param pos The position to store.
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
    }

    // Find a char starting at the current input position.
    // @param pat The char/string to search for.
    // @return -1 if not found.
    pub fn find(&self, ch: char) -> Option<usize> {
        let slice = &self.input[self.input_position..2];
        slice.find(ch).map(|x| x + self.input_position)
    }

    // Find a char starting at the position provided.
    // @param pat The char/string to search for.
    // @param start_pos The index to start at.
    // @return -1 if not found.
    pub fn find_at(&self, ch: char, start_pos: usize) -> Option<usize> {
        self.input[start_pos..].find(ch)
    }

    // Find the string starting ath the current input position.
    // @param str The string to search for.
    // @return -1 if not found.
    pub fn find_str_at_input_position(&self, strg: &str) -> Option<usize> {
        self.input[self.input_position..].find(strg)
    }

    // Find the string starting at the position provided.
    // @param strg The string to search for.
    // @param start_pos The index to start the search.
    // @return -1 if not found.
    pub fn find_str_at(&self, strg: &str, start_pos: usize) -> Option<usize> {
        self.input[start_pos..].find(strg)
    }

    // Find a char starting at the position provided. The char must not be
    // inside a quoted string (single or double).
    // @param ch The char to search for.
    // @param start_pos The index to start at
    // @param quotation_char The current quotation char. Must be ' or ",
    // otherwise will be ignored.
    pub fn find_out_of_quotes(&mut self, ch: char, start_pos: usize) -> Option<usize> {
        self.find_out_of_quotes_char(ch, start_pos, 0 as char)
    }

    // Find a char starting at the position provided. The char must not be
    // inside a quoted string.  (single or double)
    // @param ch The char to search for.
    // @param startPos The index to start at.
    // @param quotationChar The current quotation char. Must be ' or ",
    // otherwise will be ignored.
    // @return -1 if not found
    pub fn find_out_of_quotes_char(
        &mut self,
        ch: char,
        start_pos: usize,
        quotation_char: char,
    ) -> Option<usize> {
        let close_bracket_index = self.find_at(ch, start_pos + 1);
        let mut quotation_char: char = quotation_char;
        match close_bracket_index {
            Some(index) => {
                //TODO fix unwrap
                let tag_code = self
                    .get_substring(start_pos, index + 1)
                    .unwrap()
                    .to_string();
                for i in 0..tag_code.len() {
                    //TODO fix unwrap
                    let current_char: char = tag_code.bytes().nth(i).unwrap() as char;
                    let previous_tag_index = if i > 0 { i - 1 } else { 0 };
                    //TODO fix unwrap
                    let previous_tag: char =
                        tag_code.bytes().nth(previous_tag_index).unwrap() as char;
                    if quotation_char == 0 as char && (current_char == '\'')
                        || (current_char == '\"')
                    {
                        // I'm entering inside a quoted string. Set quotation_char.
                        quotation_char = current_char;
                        self.count_lines_to(start_pos + 1);
                    } else if current_char == quotation_char && previous_tag != '\\' {
                        // I'm out of quotes, reset quotationChar.
                        quotation_char = 0 as char;
                    } else if current_char == ch && quotation_char != (0 as char) {
                        return self.find_out_of_quotes_char(ch, index + 1, quotation_char);
                    }
                }
            }
            None => {
                if quotation_char != 0 as char {
                    //TODO: throw error
                    //println!( "Opening/closing quote not found for quote at " +
                    //"(line " + getLineNumber() + ", column " + getColumnNumber() +
                    //")", startPos);
                    println!(
                        "Opening/closing quote not found \
                        for quote at (line {}, column {}) as offset {}.",
                        0, 0, start_pos
                    );
                }
            }
        }
        close_bracket_index
    }

    // Position the reader at the index provided. Could be anywhere within
    // the data.
    // @param pos The new current position.
    pub fn set_position(&mut self, pos: usize) {
        self.input_position = pos;
    }

    // Get the column number. Note: The column number depends on you calling
    // countLinesTo(pos). It is not necessarily the column number matching the
    // current position in the stream.
    // @return column_number
    pub fn get_column_number(&self) -> usize {
        self.column_number
    }

    // Get the line number. Note: The line number depends on you calling
    // countLinesTo(pos). It is not necessarily the line number matching the
    // current position in the stream.
    // @return line number
    pub fn get_line_number(&self) -> usize {
        self.line_number
    }

    // Get the number of character read from the source resource. The whole
    // content, not just until the current position.
    // @return Size of the data.
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
        let mut fully_buffered_reader = FullyBufferedReader::new_from_string(test_tag.to_string());
        let position = fully_buffered_reader.find_out_of_quotes('>', 0).unwrap();
        println!("> is at {}", position);
        assert_eq!('>', test_tag.as_bytes()[position] as char);
    }
}
