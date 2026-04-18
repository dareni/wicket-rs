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

pub trait FormatParam: Debug {}
impl<T> FormatParam for T where T: Debug {}

// Helper function updated to format parameters and conditionally strip quotes
fn format_msg(msg: &str, params: &[&dyn Debug]) -> String {
    let mut formatted = msg.to_string();
    for param in params {
        // Find the first placeholder ({}) or (%s) and replace it.
        if let Some(pos) = formatted.find("{}").or_else(|| formatted.find("%s")) {
            let (before, after) = formatted.split_at(pos);
            let placeholder_len = 2; // The length of "{}" or "%s"
            let mut replacement = format!("{:?}", param);
            // if should_strip_quotes
            if replacement.starts_with('"') && replacement.ends_with('"') && replacement.len() >= 2
            {
                replacement = replacement[1..replacement.len() - 1].to_string();
            } else if replacement.starts_with('\"')
                && replacement.ends_with('\"')
                && replacement.len() >= 4
            {
                replacement = replacement[2..replacement.len() - 2].to_string();
            }

            formatted = format!("{}{}{}", before, replacement, &after[placeholder_len..]);
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
pub fn not_empty_collection<'a, T, C>(
    collection: &'a C,
    message: &str,
    params: &[&dyn Debug],
) -> Result<&'a C>
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
/// Reley on the collection being able to be converted to a slice which implements is_empty().
pub fn not_empty_named_collection<'a, T, C>(collection: &'a C, name: &'static str) -> Result<&'a C>
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
    is_true(!argument, msg, params)
    // if argument {
    //     let formatted_message = format_msg(msg, params);
    //     Err(ArgsError {
    //         message: formatted_message,
    //     })
    // } else {
    //     Ok(argument)
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn not_none_test() {
        let result = not_none(Some(0), "P1");
        assert!(result.is_ok());
        let result = not_none(None as Option<u8>, "P1");
        assert!(result.is_err());
        assert_eq!(
            "Argument 'P1' may not be empty.",
            result.unwrap_err().message
        );
    }

    #[test]
    fn not_empty_char_sequence_test() {
        let result = not_empty_char_sequence("abcd", "P1");
        assert!(result.is_ok());
        let result = not_empty_char_sequence("", "P1");
        assert!(result.is_err());
        assert_eq!(
            "Argument 'P1' may not be empty.",
            result.unwrap_err().message
        );
    }

    #[test]
    fn not_empty_str_test() {
        let result = not_empty_str("abc", "P1");
        assert!(result.is_ok());
        let result = not_empty_str("", "P1");
        assert!(result.is_err());
        assert_eq!(
            "Argument 'P1' may not be empty.",
            result.unwrap_err().message
        );
    }

    #[test]
    fn not_empty_collection_test() {
        let mut test_vec: Vec<usize> = Vec::new();
        let result = not_empty_named_collection(&test_vec, "column");
        assert!(result.is_err());

        test_vec.push(1);
        let result = not_empty_named_collection(&test_vec, "column");
        assert!(result.is_ok());

        let test_array = [0; 0];
        let result = not_empty_named_collection(&test_array, "list");
        assert!(result.is_err());

        let test_array = [0];
        let result = not_empty_named_collection(&test_array, "list");
        assert!(result.is_ok());
    }

    #[test]
    fn within_range_test() {
        let result = within_range(&1, &3, 2, "P1");
        assert!(result.is_ok());
        let result = within_range(&1, &3, 4, "P1");
        assert!(result.is_err());
        assert_eq!(
            "Argument 'P1' must have a value within [1,3], but was 4",
            result.unwrap_err().message
        );
    }

    #[test]
    fn is_true_test() {
        let arg = true;
        let result = is_true(arg, "arg is not true", &[]);
        assert!(result.is_ok());
        let arg = false;
        let result = is_true(arg, "{} arg is not true.", &[&"P1"]);
        assert!(result.is_err());
        assert_eq!("P1 arg is not true.", result.unwrap_err().message);
    }

    #[test]
    fn is_false_test() {
        let arg = false;
        let result = is_false(arg, "arg is not false", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn format_test() {
        let param: Vec<&dyn Debug> = vec![&1, &2, &3, &"abcd", &true];

        assert_eq!(
            "Params = 1, 2, 3, abcd, true.",
            format_msg("Params = {}, {}, {}, {}, {}.", param.as_slice())
        );

        let param = "world";
        assert_eq!("Hello world", format_msg("Hello {}", &[&param]));

        #[derive(Debug)]
        struct Blah {
            _x: i8,
        }
        let blah = Blah { _x: 8 };

        assert_eq!(
            "blah is : Blah { _x: 8 }",
            format_msg("blah is : {}", &[&blah])
        );
    }
}
