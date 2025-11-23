use std::borrow::Cow;
use std::fmt;
use std::num::ParseIntError;

use once_cell::sync::Lazy;
use regex::{Error as RegexError, Regex, RegexBuilder};
use thiserror::Error;

/// Simplify static Pattern creation boiler plate; lazy construction once shared everywhere.
macro_rules! static_meta {
    ($name:ident, $re:expr) => {
        pub static $name: Lazy<Pattern> = Lazy::new(|| Pattern::new(Cow::Borrowed($re)));
    };
}

// Static regex string patterns.
pub static _DOUBLE_QUOTED_STRING: &str = r#""[^"]*?""#;
pub static _SINGLE_QUOTED_STRING: &str = r#"'[^']*?'"#;
pub static _STRING: Lazy<String> = Lazy::new(|| {
    format!(
        r"(?:[\w\-\.]+|{}{}{}{}",
        _DOUBLE_QUOTED_STRING, "|", _SINGLE_QUOTED_STRING, ")"
    )
});
pub static _OPTIONAL_STRING: Lazy<String> = Lazy::new(|| format!("{}{}", _STRING.as_str(), "?"));
pub static _VARIABLE_NAME: &str = "[A-Za-z_][A-Za-z0-9_-]*";
/// Allow non xml char '@' for VueJS (see https://www.w3.org/TR/REC-xml/#NT-NameStartChar).
pub static _XML_NAME: &str = r"[A-Za-z_:@][A-Za-z0-9_.-]*";

// Cached compiled regex.
static_meta!(WHITESPACE, r"\s+");
static_meta!(OPTIONAL_WHITESPACE, r"\s*");
static_meta!(NON_WORD, r"\W+");
static_meta!(COMMA, r",");
static_meta!(COLON, r":");
static_meta!(SEMICOLON, r";");
static_meta!(SLASH, r"/");
static_meta!(BACKSLASH, r"\\");
static_meta!(DOT, r"\.");
static_meta!(PLUS, r"\+");
static_meta!(MINUS, r"-");
static_meta!(DASH, r"-");
static_meta!(UNDERSCORE, r"_");
static_meta!(AMPERSAND, r"&");
static_meta!(PERCENT, r"%");
static_meta!(DOLLAR_SIGN, r"\$");
static_meta!(POUND_SIGN, r"#");
static_meta!(AT_SIGN, r"@");
static_meta!(EXCLAMATION_POINT, r"!");
static_meta!(TILDE, r"~");
static_meta!(EQUALS, r"=");
static_meta!(STAR, r"\*");
static_meta!(PIPE, r"\|");
static_meta!(LEFT_PAREN, r"\(");
static_meta!(RIGHT_PAREN, r"\)");
static_meta!(LEFT_CURLY, r"\{");
static_meta!(RIGHT_CURLY, r"\}");
static_meta!(LEFT_SQUARE, r"\[");
static_meta!(RIGHT_SQUARE, r"\]");
static_meta!(DIGIT, r"\d");
static_meta!(DIGITS, r"\d+");
static_meta!(INTEGER, r"-?\d+");
static_meta!(FLOATING_POINT_NUMBER, r"-?\d+\.?\d*|-?\.\d+");
static_meta!(POSITIVE_INTEGER, r"\d+");
static_meta!(HEXADECIMAL_DIGIT, r"[0-9a-fA-F]");
static_meta!(HEXADECIMAL_DIGITS, r"[0-9a-fA-F]+");
static_meta!(ANYTHING, r".*");
static_meta!(ANYTHING_NON_EMPTY, r".+");
static_meta!(WORD, r"\w+");
static_meta!(OPTIONAL_WORD, r"\w*");

pub static VARIABLE_NAME: Lazy<Pattern> = Lazy::new(|| Pattern::new(Cow::Borrowed(_VARIABLE_NAME)));
pub static XML_ELEMENT_NAME: Lazy<Pattern> = Lazy::new(|| Pattern::new(Cow::Borrowed(_XML_NAME)));
pub static XML_ATTRIBUTE_NAME: Lazy<Pattern> = Lazy::new(|| Pattern::new(Cow::Borrowed(_XML_NAME)));
pub static PERL_INTERPOLATION: Lazy<Pattern> =
    Lazy::new(|| Pattern::new(Cow::Owned(format!(r"\$\{{{}\}}", _VARIABLE_NAME))));
pub static DOUBLE_QUOTED_STRING: Lazy<Pattern> =
    Lazy::new(|| Pattern::new(Cow::Borrowed(_DOUBLE_QUOTED_STRING)));
pub static STRING: Lazy<Pattern> = Lazy::new(|| Pattern::new(Cow::Borrowed(_STRING.as_ref())));
pub static OPTIONAL_STRING: Lazy<Pattern> =
    Lazy::new(|| Pattern::new(Cow::Borrowed(_OPTIONAL_STRING.as_ref())));

/// Uses named groups 'key' and 'value'
pub static STRING_VARIABLE_ASSIGNMENT: Lazy<Pattern> =
    Lazy::new(|| Pattern::new(Cow::Owned(get_variable_assignment_pattern::<&str>(None))));

/// Uses named groups 'key' and 'value'
pub static INTEGER_VARIABLE_ASSIGNMENT: Lazy<Pattern> =
    Lazy::new(|| Pattern::new(Cow::Owned(get_integer_assignment_pattern())));

pub struct Pattern {
    source: Cow<'static, str>,
    regex: Regex,
}

impl Pattern {
    pub fn new(source: Cow<'static, str>) -> Self {
        let regex = Regex::new(source.as_ref())
            .unwrap_or_else(|e| panic!("Could not compile regex: {} cause: {}", source, e));
        Pattern { source, regex }
    }

    pub fn new_with_flags(source: Cow<'static, str>, flags: &RegexFlags) -> Self {
        let mut regex_builder = RegexBuilder::new(source.as_ref());
        let regex_builder = regex_builder
            .case_insensitive(flags.case_insensitive)
            .multi_line(flags.multi_line)
            .dot_matches_new_line(flags.dot_matches_new_line)
            .ignore_whitespace(flags.ignore_whitespace);

        let regex = regex_builder
            .build()
            .unwrap_or_else(|e| panic!("Could not compile regex: {} cause: {}", source, e));

        Pattern { source, regex }
    }

    pub fn as_str(&self) -> &str {
        self.source.as_ref()
    }
    pub fn get_regex(&self) -> &Regex {
        &self.regex
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Writes the raw source string to the formatter
        write!(f, "{}", self.source)
    }
}

pub trait PatternOps: Sized + std::fmt::Display {
    fn append_pattern<T: AsRef<str>>(self, suffix: T) -> String;
    fn append_named_capture<T: AsRef<str>>(self, pattern: T, pattern_name: &'static str) -> String;
    fn capture_group_named(self, pattern_name: &'static str) -> String {
        format!("(?P<{}>{})", pattern_name, self)
    }
    fn capture_group_unnamed(self) -> String {
        format!("({})", self)
    }
    /// A non catpure group.
    fn make_pattern_group(self) -> String {
        format!("(?:{})", self)
    }
    // An optional non capture group.
    fn make_pattern_optional(self) -> String {
        format!("(?:{})?", self)
    }
}

impl PatternOps for String {
    fn append_pattern<T: AsRef<str>>(mut self, suffix: T) -> String {
        let suffix_str = suffix.as_ref();
        self.push_str(suffix_str);
        self
    }

    fn append_named_capture<T: AsRef<str>>(
        mut self,
        pattern: T,
        pattern_name: &'static str,
    ) -> String {
        let named_group_exp = format!("(?P<{}>{})", pattern_name, pattern.as_ref());
        self.push_str(&named_group_exp);
        self
    }
}

/// Parsing operation exceptions.
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Exception raised by regex engine: {0}")]
    Regex(#[from] RegexError),
    /// Indicates that the current pattern didn't match.
    #[error("No match found")]
    NoMatch,
    #[error("Input was not a valid integer: {0}")]
    ParseIntError(#[from] ParseIntError),
}

/// Flags corresponding to `regex::RegexBuilder` options.
#[derive(Default, Clone, Copy)]
pub struct RegexFlags {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub ignore_whitespace: bool,
}

pub mod capture_name {
    pub static KEY: &str = "key";
    pub static VALUE: &str = "value";
}

// Variable assignment pattern build.

/// The optional namespace like "namespace:*[:*]"
pub fn get_namespace_pattern() -> String {
    let namespace = String::with_capacity(60)
        .append_pattern(VARIABLE_NAME.as_str())
        .append_pattern(COLON.as_str())
        .append_pattern(
            VARIABLE_NAME
                .to_string()
                .append_pattern(COLON.as_str())
                .make_pattern_optional(),
        );
    namespace.make_pattern_optional()
}
/// The key (lvalue) like "name" or "namespace:name" or "namespace:name:subname"
pub fn get_key_group_pattern() -> String {
    let key_group = get_namespace_pattern()
        .append_pattern(XML_ATTRIBUTE_NAME.as_str())
        .capture_group_named(capture_name::KEY);
    key_group
}
/// Parses key value assignment statements like "foo=bar" but also supporting namespaces like
/// "wicket:foo=bar". However the 'key' value returned will contain "wicket:foo". It does not
/// separate namespace and name.
pub fn get_variable_assignment_pattern<T: AsRef<str>>(value_pattern_opt: Option<T>) -> String {
    let key_group = get_key_group_pattern();
    let value_pattern = match value_pattern_opt {
        Some(val) => val.as_ref().to_string(),
        None => STRING.as_str().to_string(),
    };
    let value_group =
        String::with_capacity(50).append_named_capture(value_pattern, capture_name::VALUE);
    let variable_assignment = String::with_capacity(60)
        .append_pattern(OPTIONAL_WHITESPACE.as_str())
        .append_pattern(EQUALS.as_str())
        .append_pattern(OPTIONAL_WHITESPACE.as_str())
        .append_pattern(value_group);

    let full_key_value_assignment_pattern = String::with_capacity(180)
        .append_pattern(OPTIONAL_WHITESPACE.as_str())
        .append_pattern(key_group)
        .append_pattern(
            variable_assignment
                .append_pattern(OPTIONAL_WHITESPACE.as_str())
                .make_pattern_optional(),
        );

    full_key_value_assignment_pattern
}

// Integer assignment pattern build.

/// Parses integer variable assignments, such as "x = 9" or "x=9".
pub fn get_integer_assignment_pattern() -> String {
    let variable_group_pattern = String::with_capacity(100)
        .append_pattern(VARIABLE_NAME.as_str())
        .capture_group_named(capture_name::KEY);
    let value_group_pattern =
        String::with_capacity(100).append_named_capture(INTEGER.as_str(), capture_name::VALUE);

    let pattern = variable_group_pattern
        .append_pattern(OPTIONAL_WHITESPACE.as_str())
        .append_pattern(EQUALS.as_str())
        .append_pattern(OPTIONAL_WHITESPACE.as_str())
        .append_pattern(value_group_pattern);
    pattern
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn simple() {
        let key_value_cap = VARIABLE_NAME
            .to_string()
            .capture_group_named(capture_name::KEY)
            .append_pattern(OPTIONAL_WHITESPACE.as_str())
            .append_pattern(EQUALS.as_str())
            .append_pattern(OPTIONAL_WHITESPACE.as_str())
            .append_named_capture(INTEGER.as_str(), capture_name::VALUE);

        let key_value_pattern: Pattern = Pattern::new(Cow::Owned(key_value_cap));
        let caps_opt = key_value_pattern.regex.captures("foo = 9");
        assert!(caps_opt.is_some());
        let caps = caps_opt.unwrap();
        assert_eq!(caps.name(capture_name::KEY).unwrap().as_str(), "foo");
        assert_eq!(caps.name(capture_name::VALUE).unwrap().as_str(), "9");
    }

    #[test]
    fn simple_lazy_pattern() {
        let pattern = &STRING_VARIABLE_ASSIGNMENT;
        let captures_opt = pattern.get_regex().captures("foo=9");

        assert!(captures_opt.is_some());
        let captures = captures_opt.unwrap();
        assert_eq!(captures.name(capture_name::KEY).unwrap().as_str(), "foo");
        assert_eq!(captures.name(capture_name::VALUE).unwrap().as_str(), "9");
    }
}
