use regex::Regex;
use std::fmt;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
/// Helper that creates a `MetaPattern` from a static string at compile time.
/// The returned value is a `Lazy<MetaPattern>` so it is constructed only once
/// and then shared everywhere it is used.
///
macro_rules! static_meta {
    ($name:ident, $re:expr) => {
        pub static $name: Lazy<MetaPattern> = Lazy::new(|| MetaPattern::meta_from_str($re));
    };
}

/// A meta‑pattern that can be a single regex or a composition of other meta‑patterns.
#[derive(Clone)]
pub struct MetaPattern {
    /// Raw regex string (either supplied directly or built from children).
    raw: Arc<String>,
    /// Optional compiled regex – created lazily.
    compiled: Arc<Mutex<Option<Regex>>>,
    /// Optional child patterns for composite patterns.
    children: Option<Arc<Vec<MetaPattern>>>,
}

impl MetaPattern {
    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    /// Construct from a raw regex string.
    pub fn meta_from_str<S: Into<String>>(s: S) -> Self {
        let raw = Arc::new(s.into());
        Self {
            raw,
            compiled: Arc::new(Mutex::new(None)),
            children: None,
        }
    }

    /// Copy constructor – clones the internal state.
    pub fn from_meta(other: &MetaPattern) -> Self {
        other.clone()
    }

    /// Construct from a slice of `MetaPattern`s (composite pattern).
    pub fn from_slice(slice: &[MetaPattern]) -> Self {
        let children = Arc::new(slice.to_vec());
        // Build the concatenated raw string lazily.
        let raw = Arc::new(String::new()); // placeholder; will be filled on compile
        Self {
            raw,
            compiled: Arc::new(Mutex::new(None)),
            children: Some(children),
        }
    }

    // -------------------------------------------------------------------------
    // Core API
    // -------------------------------------------------------------------------

    /// Returns the compiled `Regex`, compiling it on first use.
    /// `flags` can be a combination of the `regex::RegexBuilder` options.
    pub fn regex(&self, flags: Option<RegexFlags>) -> Regex {
        // Fast path – already compiled.
        if let Some(re) = self.compiled.lock().unwrap().as_ref() {
            return re.clone();
        }

        // Build the pattern string.
        let pattern = if let Some(children) = &self.children {
            // Concatenate child patterns.
            let mut buf = String::new();
            for child in children.iter() {
                buf.push_str(&child.get_raw());
            }
            // Store the concatenated string for future calls.

            // let mut raw_mut = Arc::get_mut(&mut self.raw.clone()).unwrap();
            let mut tmp = self.raw.clone();
            let raw_mut = Arc::get_mut(&mut tmp).unwrap();
            *raw_mut = buf.clone();
            buf
        } else {
            //TODO: check
            // (**self.raw).clone()
            (**self.raw).to_string().clone()
        };

        // Apply flags via RegexBuilder.
        let mut builder = regex::RegexBuilder::new(&pattern);
        if let Some(f) = flags {
            builder
                .case_insensitive(f.case_insensitive)
                .multi_line(f.multi_line)
                .dot_matches_new_line(f.dot_matches_new_line)
                .ignore_whitespace(f.ignore_whitespace);
        }
        let compiled = builder.build().expect("Invalid regex pattern");

        // Cache the compiled regex.
        *self.compiled.lock().unwrap() = Some(compiled.clone());
        compiled
    }

    /// Create a matcher (iterator over captures) for the given input.
    pub fn matcher<'a>(&'a self, input: &'a str, flags: Option<RegexFlags>) -> regex::Captures<'a> {
        self.regex(flags).captures(input).expect("No match")
    }

    /// Returns the raw pattern string (concatenated if composite).
    fn get_raw(&self) -> String {
        if let Some(children) = &self.children {
            let mut buf = String::new();
            for child in children.iter() {
                buf.push_str(&child.get_raw());
            }
            buf
        } else {
            //TODO: check
            // (**self.raw).clone()
            self.raw.clone().to_string()
        }
    }
}

// -------------------------------------------------------------------------
// Helper for flag handling
// -------------------------------------------------------------------------

/// Flags that correspond to `regex::RegexBuilder` options.
#[derive(Default, Clone, Copy)]
pub struct RegexFlags {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub ignore_whitespace: bool,
}

// -----------------------------------------------------------------------------
// Debug implementation (mirrors Java's toString)
// -----------------------------------------------------------------------------
impl fmt::Debug for MetaPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.get_raw())
    }
}

// -----------------------------------------------------------------------------
// static regex patterns
// -----------------------------------------------------------------------------
pub static _DOUBLE_QUOTED_STRING: &str = r#""[^"]*?""#;
pub static _SINGLE_QUOTED_STRING: &str = r#"'[^']*?'"#;
pub static _STRING: Lazy<String> = Lazy::new(|| {
    format!(
        r"(?:[\w\-\.\]+|{}{}{}{}",
        _DOUBLE_QUOTED_STRING, "|", _SINGLE_QUOTED_STRING, ")"
    )
});
pub static _OPTIONAL_STRING: Lazy<String> = Lazy::new(|| format!("{}{}", _STRING.as_str(), "?"));
pub static _VARIABLE_NAME: &str = "[A-Za-z_][A-Za-z0-9_-]*";
pub static _XML_NAME: &str = r"[A-Za-z_:@][A-Za-z0-9_.-]*";
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
static_meta!(DOLLAR_SIGN, r"$");
static_meta!(POUND_SIGN, r"#");
static_meta!(AT_SIGN, r"@");
static_meta!(EXCLAMATION_POINT, r"!");
static_meta!(TILDE, r"~");
static_meta!(EQUALS, r"=");
static_meta!(STAR, r"\*");
static_meta!(PIPE, r"\|");
static_meta!(LEFT_PAREN, r"$");
static_meta!(RIGHT_PAREN, r"$");
static_meta!(LEFT_CURLY, r"\{");
static_meta!(RIGHT_CURLY, r"\}");
static_meta!(LEFT_SQUARE, r"$$");
static_meta!(RIGHT_SQUARE, r"$$");
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
pub static VARIABLE_NAME: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(_VARIABLE_NAME));
pub static XML_ELEMENT_NAME: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(_XML_NAME));
pub static XML_ATTRIBUTE_NAME: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(_XML_NAME));
pub static PERL_INTERPOLATION: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(format!(r"\$\{{{}\}}", _VARIABLE_NAME)));
pub static DOUBLE_QUOTED_STRING: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(_DOUBLE_QUOTED_STRING));
pub static STRING: Lazy<MetaPattern> = Lazy::new(|| MetaPattern::meta_from_str(_STRING.as_str()));
pub static OPTIONAL_STRING: Lazy<MetaPattern> =
    Lazy::new(|| MetaPattern::meta_from_str(_OPTIONAL_STRING.as_str()));
// ----------------------------------------------
