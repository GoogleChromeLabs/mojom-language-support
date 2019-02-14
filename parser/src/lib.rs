#[macro_use]
extern crate nom;
#[macro_use]
extern crate nom_locate;

use nom::types::CompleteStr;

/// Span represents a slice of input &str along with its position.
type Span<'a> = nom_locate::LocatedSpan<CompleteStr<'a>>;

mod identifier;
mod interface;

pub enum Definition<'a> {
    Interface(interface::Interface<'a>),
}

/// parses `input`.
pub fn parse(input: &str) -> Definition {
    let input = Span::new(input.into());
    let res = interface::interface(input).unwrap().1;
    Definition::Interface(res)
}
