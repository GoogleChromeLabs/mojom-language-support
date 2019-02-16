#[macro_use]
extern crate pest_derive;

pub use pest::error::Error;
pub use pest::error::LineColLocation;

pub use pest::Span;

mod mojom;

pub use mojom::*;
