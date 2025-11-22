use regex::Captures;

use crate::wicket::util::parse::metapattern::capture_name;
use crate::wicket::util::parse::metapattern::ParserError;
use crate::wicket::util::parse::metapattern::INTEGER_VARIABLE_ASSIGNMENT;
use crate::wicket::util::parse::metapattern::STRING_VARIABLE_ASSIGNMENT;

pub struct StringVariableAssignmentParser<'a> {
    captures: Option<Captures<'a>>,
}

impl<'a> StringVariableAssignmentParser<'a> {
    pub fn new<T: AsRef<str> + ?Sized>(haystack: &'a T) -> Self {
        let captures_option = STRING_VARIABLE_ASSIGNMENT
            .get_regex()
            .captures(haystack.as_ref());
        Self {
            captures: captures_option,
        }
    }

    pub fn capture<T: AsRef<str> + ?Sized>(&mut self, haystack: &'a T) {
        self.captures = STRING_VARIABLE_ASSIGNMENT
            .get_regex()
            .captures(haystack.as_ref());
    }

    pub fn is_capture(&self) -> bool {
        self.captures.is_some()
    }

    pub fn get_key(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::KEY) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_value(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }
}

pub struct IntegerVariableAssignmentParser<'a> {
    captures: Option<Captures<'a>>,
}

impl<'a> IntegerVariableAssignmentParser<'a> {
    pub fn new<T: AsRef<str> + ?Sized>(haystack: &'a T) -> Self {
        let captures_option = INTEGER_VARIABLE_ASSIGNMENT
            .get_regex()
            .captures(haystack.as_ref());
        Self {
            captures: captures_option,
        }
    }

    pub fn capture<T: AsRef<str> + ?Sized>(&mut self, haystack: &'a T) {
        self.captures = INTEGER_VARIABLE_ASSIGNMENT
            .get_regex()
            .captures(haystack.as_ref());
    }

    pub fn is_capture(&self) -> bool {
        self.captures.is_some()
    }

    pub fn get_key(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::KEY) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_value(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_int_value(&self) -> Result<i64, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str().parse::<i64>()?),
                None => Err(ParserError::NoMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::wicket::util::parse::metapattern::parsers::{
        IntegerVariableAssignmentParser, StringVariableAssignmentParser,
    };

    #[test]
    fn string_vaiable_assignment_parser() {
        let mut parser = StringVariableAssignmentParser::new("foo = ");
        assert!(parser.is_capture());
        assert_eq!(parser.get_key().unwrap(), "foo");
        assert!(parser.get_value().is_err());

        parser.capture("foo = 9");
        assert_eq!(parser.get_key().unwrap(), "foo");
        assert_eq!(parser.get_value().unwrap(), "9");
    }
    #[test]
    fn integer_vaiable_assignment_parser() {
        let mut parser = IntegerVariableAssignmentParser::new("foo = ");
        assert!(!parser.is_capture());
        parser.capture("foo = 9");
        assert_eq!(parser.get_key().unwrap(), "foo");
        assert_eq!(parser.get_int_value().unwrap(), 9);
    }
}
