pub mod behavior;
pub mod core;
pub mod markup;
pub mod settings;

pub trait Component {}
pub struct MarkupContainer {}

#[derive(Default)]
pub struct MarkupIdGenerator {}
