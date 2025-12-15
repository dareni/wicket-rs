use regex::Captures;

use crate::wicket::util::parse::metapattern::capture_name;
use crate::wicket::util::parse::metapattern::ParserError;
use crate::wicket::util::parse::metapattern::Pattern;
use crate::wicket::util::parse::metapattern::INTEGER_VARIABLE_ASSIGNMENT;
use crate::wicket::util::parse::metapattern::STRING_VARIABLE_ASSIGNMENT;
use crate::wicket::util::parse::metapattern::XML_TAG_NAME;

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

    // TODO: Move to trait
    pub fn is_capture(&self) -> bool {
        self.captures.is_some()
    }

    // TODO: Move to trait
    // The byte index of the capture end point in the haystack.
    pub fn end(&self) -> usize {
        match &self.captures {
            Some(cap) => cap.get_match().end(),
            None => 0,
        }
    }

    // TODO: Write a function taking a closure for working with captures_iter() and exposing the
    // parser functions in the closure. This is currently achieved by stepping through
    // the haystack with end() but is not efficient compared with captures_iter().

    /// Get the capture_name::KEY
    pub fn get_key(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::KEY) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    /// Get the capture_name::VALUE
    pub fn get_value(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
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

    pub fn end(&self) -> usize {
        match &self.captures {
            Some(cap) => cap.get_match().end(),
            None => 0,
        }
    }

    pub fn get_key(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::KEY) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_value(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_int_value(&self) -> Result<i64, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::VALUE) {
                Some(val) => Ok(val.as_str().parse::<i64>()?),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }
}

pub struct TagNameParser<'a> {
    captures: Option<Captures<'a>>,
}

impl<'a> TagNameParser<'a> {
    pub fn new<T: AsRef<str> + ?Sized>(haystack: &'a T) -> Self {
        let captures_option = XML_TAG_NAME.get_regex().captures(haystack.as_ref());
        Self {
            captures: captures_option,
        }
    }

    pub fn capture<T: AsRef<str> + ?Sized>(&mut self, haystack: &'a T) {
        self.captures = XML_TAG_NAME.get_regex().captures(haystack.as_ref());
    }

    pub fn is_capture(&self) -> bool {
        self.captures.is_some()
    }

    pub fn end(&self) -> usize {
        match &self.captures {
            Some(cap) => cap.get_match().end(),
            None => 0,
        }
    }

    pub fn get_name(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::NAME) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }

    pub fn get_namespace(&self) -> Result<&'a str, ParserError> {
        match &self.captures {
            Some(cap) => match cap.name(capture_name::NAMESPACE_NAME) {
                Some(val) => Ok(val.as_str()),
                None => Err(ParserError::NoGroupMatch),
            },
            None => Err(ParserError::NoMatch),
        }
    }
}

pub struct ListParser<'a> {
    pattern: &'a Pattern,
}

impl<'a> ListParser<'a> {
    /// A haystack containing a repeating element matching the group capture of Pattern.
    /// The delimiter for the final element may be EOL. For example the delimiting
    /// element for CSV is  (?:,|?)
    pub fn new(single_capture_pattern: &'a Pattern) -> Self {
        Self {
            pattern: single_capture_pattern,
        }
    }

    pub fn get_matches<T: AsRef<str> + ?Sized>(
        &self,
        haystack: &'a T,
    ) -> Result<Vec<String>, ParserError> {
        Ok(self
            .pattern
            .get_regex()
            .captures_iter(haystack.as_ref())
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str().to_owned())
            .collect::<Vec<String>>())
    }

    pub fn get_match_slices<'h, T: AsRef<str> + ?Sized>(
        &self,
        haystack: &'h T,
    ) -> Result<Vec<&'h str>, ParserError> {
        Ok(self
            .pattern
            .get_regex()
            .captures_iter(haystack.as_ref())
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str())
            .collect::<Vec<&'h str>>())
    }
}

#[cfg(test)]
mod test {

    use crate::wicket::util::parse::metapattern::parsers::{
        IntegerVariableAssignmentParser, ListParser, StringVariableAssignmentParser, TagNameParser,
    };
    use crate::wicket::util::parse::metapattern::{get_tag_name_pattern, COMMA_SEPARATED_VARIABLE};

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
    #[test]
    fn list_parser() {
        let parser = ListParser::new(&COMMA_SEPARATED_VARIABLE);

        let vec = parser.get_match_slices("a,b,c");
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec!["a", "b", "c"]);
        let vec = parser.get_match_slices("a,b,c,");
        assert!(vec.is_ok());
        assert_eq!(vec.unwrap(), vec!["a", "b", "c"]);
    }

    #[test]
    fn tag_parser() {
        println!("regex:{}", get_tag_name_pattern());
        let tag = "name";
        let mut parser = TagNameParser::new(&tag);
        assert!(parser.is_capture());
        assert_eq!(&tag, &(parser.get_name().unwrap()));
        assert!(parser.get_namespace().is_err());

        let tag = "namespace:name";
        parser.capture(tag);
        assert!(parser.is_capture());
        assert_eq!(&"name", &(parser.get_name().unwrap()));
        assert_eq!(&"namespace", &(parser.get_namespace().unwrap()));

        let tag = "namespace:";
        parser.capture(&tag);
        assert!(!parser.is_capture());

        // leading : is allowed according to https://www.w3.org/TR/REC-xml/#NT-NameStartChar
        let tag = ":names";
        parser.capture(tag);
        assert!(parser.is_capture());

        let tag = "tag ";
        parser.capture(tag);
        assert!(parser.is_capture())
    }
}
