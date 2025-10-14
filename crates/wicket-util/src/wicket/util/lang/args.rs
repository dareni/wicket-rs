use std::borrow::Borrow;
use std::cmp::PartialOrd;
use std::fmt::{Debug, Display, Formatter};

// ====================================================================
// Custom Error Type (ArgsError)
// ====================================================================

/// The error type for all argument assertion failures.
#[derive(Debug, Clone)]
pub struct ArgsError {
    message: String,
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Argument Assertion Failed: {}", self.message)
    }
}

// Allow conversion to a standard error
impl std::error::Error for ArgsError {}

// ====================================================================
// INTERNAL HELPER FUNCTIONS
// ====================================================================

/// Helper to replicate the custom string formatting used in the Java code.
fn format_msg(msg: &str, params: &[&dyn Debug]) -> String {
    let mut formatted = msg.to_string();
    for param in params {
        // Find the first placeholder ({}) or (%s) and replace it.
        if let Some(pos) = formatted.find("{}").or_else(|| formatted.find("%s")) {
            let (before, after) = formatted.split_at(pos);
            let placeholder_len = 2; // The length of "{}" or "%s"
            formatted = format!("{}{:?}{}", before, param, &after[placeholder_len..]);
        } else {
            break;
        }
    }
    formatted
}

/// Checks if a string reference is empty or contains only whitespace.
fn is_empty_or_whitespace(s: &str) -> bool {
    s.trim().is_empty()
}

// ====================================================================
// PUBLIC ARGUMENT CHECKING FUNCTIONS RETURNING RESULT
// ====================================================================

/// Class with functions for asserting conditions on arguments, returning Result on success.
/// Type alias for convenience
pub type Result<T> = std::result::Result<T, ArgsError>;

/// Checks argument is not None.
pub fn not_none<T>(argument: Option<T>, name: &str) -> Result<T> {
    match argument {
        Some(val) => Ok(val),
        None => Err(ArgsError {
            message: format!("Argument '{}' may not be empty.", name),
        }),
    }
}

/// Checks a string argument is not zero-length, and has a non-whitespace character.
pub fn not_empty_char_sequence<T>(argument: T, name: &str) -> Result<T>
where
    T: AsRef<str> + Debug,
{
    if is_empty_or_whitespace(argument.as_ref()) {
        Err(ArgsError {
            message: format!("Argument '{}' may not be empty.", name),
        })
    } else {
        Ok(argument)
    }
}

/// Checks a string reference is not empty.
pub fn not_empty_str<'a>(argument: &'a str, name: &'a str) -> Result<&'a str> {
    if is_empty_or_whitespace(argument) {
        Err(ArgsError {
            message: format!("Argument '{}' may not be empty.", name),
        })
    } else {
        Ok(argument)
    }
}

/// Checks a collection argument is not empty.
pub fn not_empty_collection<T, C>(collection: C, message: &str, params: &[&dyn Debug]) -> Result<C>
where
    C: Borrow<[T]>,
{
    if collection.borrow().is_empty() {
        let formatted_message = format_msg(message, params);
        Err(ArgsError {
            message: formatted_message,
        })
    } else {
        Ok(collection)
    }
}

/// Checks a collection argument is not empty.
pub fn not_empty_named_collection<T, C>(collection: C, name: &str) -> Result<C>
where
    C: Borrow<[T]>,
{
    not_empty_collection(collection, "Collection '{}' may not be empty.", &[&name])
}

/// Checks if argument is within a range: `min <= value <= max`.
pub fn within_range<T>(min: &T, max: &T, value: T, name: &str) -> Result<T>
where
    T: PartialOrd + Debug,
{
    if &value < min || &value > max {
        let error_msg = format!(
            "Argument '{}' must have a value within [{:?},{:?}], but was {:?}",
            name, min, max, value
        );
        Err(ArgsError { message: error_msg })
    } else {
        Ok(value)
    }
}

/// Check if argument is true.
pub fn is_true(argument: bool, msg: &str, params: &[&dyn Debug]) -> Result<bool> {
    if !argument {
        let formatted_message = format_msg(msg, params);
        Err(ArgsError {
            message: formatted_message,
        })
    } else {
        Ok(argument)
    }
}

/// Check if argument is false.
pub fn is_false(argument: bool, msg: &str, params: &[&dyn Debug]) -> Result<bool> {
    if argument {
        let formatted_message = format_msg(msg, params);
        Err(ArgsError {
            message: formatted_message,
        })
    } else {
        Ok(argument)
    }
}
