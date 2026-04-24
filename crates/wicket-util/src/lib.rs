pub mod collections;
pub mod lang;
pub mod parse;
pub mod string;

pub mod constants {
    pub mod file_ext {
        /// Standard Wicket Markup
        pub const HTML: &str = "html";
        pub const XML: &str = "xml";
        pub const XHTML: &str = "xhtml";

        /// Configuration and Localization
        pub const PROPERTIES: &str = "properties";
        pub const JSON: &str = "json";

        /// Client-side Resources
        pub const JS: &str = "js";
        pub const CSS: &str = "css";
    }
}
