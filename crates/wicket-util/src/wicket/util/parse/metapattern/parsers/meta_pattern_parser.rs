use regex::{Error as RegexError, Regex};

/// A custom error type for parsing operations.
#[derive(Debug)]
pub enum ParserError {
    /// Error from the underlying regex engine.
    Regex(RegexError),
    /// Indicates that the current pattern didn't match.
    NoMatch,
    /// Custom error for when no pattern has been set (Matcher is None).
    NoPatternSet,
}

impl From<RegexError> for ParserError {
    fn from(err: RegexError) -> Self {
        ParserError::Regex(err)
    }
}

// In the Java code, MetaPattern is used to create a Matcher.
// In Rust, we'll assume a MetaPattern would primarily be a wrapper around a compiled Regex.
// For simplicity in this base class, we'll use a direct reference to a Regex object
// for the `advance` method, but `set_pattern` will be more complex since the Java MetaPattern
// holds the Matcher state.

/// Base struct for various MetaPattern based parsers.
pub struct MetaPatternParser<'a> {
    /// The input to parse
    input: &'a str,

    /// The length of the input; no. of characters (in bytes)
    length: usize,

    /// The position (index in bytes) behind the last pattern group matched
    pos: usize,

    /// The object maintaining the regex match details.
    /// It's optional because it may not be set by the first constructor.
    // Note: Rust Matcher doesn't hold the state like Java's Matcher,
    // it's created on the fly by a search operation. We'll use Option<regex::Regex>
    // to track the *last used pattern* or perhaps keep the last matched details
    // if required by public methods.
    last_matcher: Option<regex::Match<'a>>,
}

impl<'a> MetaPatternParser<'a> {
    /// Construct the parser.
    /// You must call `advance()` to initialize the parser state.
    pub fn new(input: &'a str) -> Self {
        MetaPatternParser {
            input,
            length: input.len(), // Byte length, common for Rust str
            pos: 0,
            last_matcher: None,
        }
    }

    /// Advance parsing to the next element. The internal cursor will be moved
    /// to the end of the string matched.
    ///
    /// Note: The original Java code created a new Matcher on a subsequence.
    /// In idiomatic Rust regex, we use the `find` or `is_match_at` methods
    /// on the whole string, starting the search from `pos`.
    ///
    /// # Returns
    /// A Result indicating success (`true`) or a parsing error.
    pub fn advance(&mut self, pattern: &Regex) -> Result<bool, ParserError> {
        // Get the remaining part of the input as a slice starting from the current position
        let remaining_input = &self.input[self.pos..];

        // The Java code uses "lookingAt()" which attempts to match only at the start of the region.
        // We can achieve a similar effect by checking for a match that starts exactly at index 0
        // of the *remaining* slice, or using `find` and checking the start index.

        // Use `find` which returns the first match in the remaining text.
        if let Some(m) = pattern.find(remaining_input) {
            // Check if the match starts at the beginning of the remaining slice (index 0).
            // This is crucial to mimic the Java `lookingAt` behavior on a subsequence.
            if m.start() == 0 {
                // Yes, it does. Move the cursor.
                // The match's end index (m.end()) is relative to the `remaining_input`.
                self.pos += m.end();
                self.last_matcher = Some(m);
                return Ok(true);
            }
        }

        // Did not find the pattern at the current position.
        self.last_matcher = None;
        Ok(false)
    }

    /// Whether the last pattern matched the entire current search region.
    /// NOTE: This is complex to map directly. The Java `matches()` refers to the state
    /// of the Matcher set by `setPattern` or the *last* `advance`.
    /// Given `advance` sets `last_matcher`, this checks if the last match consumed
    /// the *entire* remaining input it was run against.
    pub fn matches(&self) -> bool {
        // This translation is an interpretation, as Java's Matcher state is complex.
        // We'll say the "match" is true if the last match consumed the entire remaining
        // input that was visible at the time of the advance call.
        if let Some(m) = &self.last_matcher {
            // If the match starts at pos 0 of the remaining string, AND
            // the length of the match equals the length of the remaining string.
            m.end() == self.input.len() - self.pos + m.start()
        } else {
            false
        }
    }

    /// Gets the last matched result.
    ///
    /// # Returns
    /// An Option containing the last `regex::Match` details.
    pub fn last_matcher(&self) -> Option<&regex::Match<'a>> {
        self.last_matcher.as_ref()
    }

    /// Whether the internal cursor has advanced to the end of the input.
    pub fn at_end(&self) -> bool {
        self.pos == self.length
    }
}
