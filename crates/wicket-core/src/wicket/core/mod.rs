pub mod behavior;
pub mod central;
pub mod markup;
pub mod settings;

pub trait Component {}
pub struct MarkupContainer {}

#[derive(Default)]
pub struct MarkupIdGenerator {}
