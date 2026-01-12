use crate::wicket::{markup::MarkupFactory, MarkupIdGenerator};

#[derive(Default)]
pub struct MarkupSettings {
    // Application default for automatically resolving hrefs.
    pub automatic_linking: bool,

    // True if multiple tabs/spaces should be compressed to a single space.
    pub compress_whitespace: bool,

    // Default markup encoding. If null, the OS default will be used.
    pub default_markup_encoding: String,

    // Factory for creating markup parsers.
    pub markup_factory: MarkupFactory,

    // If true, then throw an exception if the xml declaration is missing from the markup file.
    pub throw_exception_on_missing_xml_declaration: bool, // = false;

    // Should HTML comments be stripped during rendering?
    pub strip_comments: bool, // = false;

    // If true, wicket tags ( <wicket: ..>) and wicket:id attributes will be removed from output.
    pub strip_wicket_tags: bool, // = false;

    // If true, wicket auto-labels will always be updated (via AJAX) whenever the associated form component is.
    // The default is false (for backward compatibility).
    pub update_autolabels_together_with_form_component: bool, //   = false;

    // Generates the markup ids for the components with
    // org.apache.wicket.Component#setOutputMarkupId(boolean) #setOutputMarkupId(true)}
    pub markup_id_generator: MarkupIdGenerator,
}
