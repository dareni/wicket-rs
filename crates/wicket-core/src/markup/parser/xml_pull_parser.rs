#![allow(unused)]

static STYLE: &str = "style";
static SCRIPT: &str = "script";

enum SkipType {
    Style,
    Script,
    None,
}

impl SkipType {
    fn value(&self) -> &str {
        match *self {
            Self::Style => "style",
            Self::Script => "script",
            Self::None => "",
        }
    }
}

pub fn parse() {
    println!("parsing");
}

struct XmlPullParser<'a> {
    // Encoding of the xml.
    encoding: String,

    // A XML independent reader which loads the whole source data into memory
    // and which provides convenience methods to access the data.
    //input: FullyBufferedReader,
    //
    // Temporary variable which will hold the name of the closing tag
    skip_until_text: SkipType,
    last_text: Option<&'a str>,
}

impl<'a> XmlPullParser<'a> {
    fn new(input: String) -> Self {
        Self {
            encoding: "utf8".to_string(),
            skip_until_text: SkipType::None,
            last_text: Option::None,
        }
    }
}

trait IXmlPullParser {
    fn get_encoding<'a>(&'a self) -> &'a str;
}

enum HttpTagType {
    // next() must be called at least once for the Type to be valid
    NotInitialized,

    // <name>
    Tag,

    // Tag body in between two tags
    Body,

    // !--
    Comment,

    // <!--[if ] ... -->
    ConditionalComment,

    // <![endif]-->
    ConditionalCommentEndif,

    // <![CDATA[ .. ]]>
    Cdata,

    // <?...>
    ProcessingInstruction,

    // <!DOCTYPE ...>
    Doctype,

    //all other tags which look like <!.. >
    Special,
}
