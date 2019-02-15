#[macro_use]
extern crate nom;
#[macro_use]
extern crate nom_locate;

use nom::types::CompleteStr;

/// Span represents a slice of input &str along with its position.
pub type Span<'a> = nom_locate::LocatedSpan<CompleteStr<'a>>;

pub enum Error<'a> {
    SyntaxError(Span<'a>),
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

mod definition;
mod identifier;
mod interface;

use definition::Definition;

#[derive(Debug)]
pub struct Mojom<'a> {
    pub definitions: Vec<Definition<'a>>,
}

/// parses `input`.
pub fn parse(input: &str) -> Result<Mojom> {
    let input = Span::new(input.into());
    let definitions = definition::definitions(input)?;
    Ok(Mojom {
        definitions: definitions,
    })
}
