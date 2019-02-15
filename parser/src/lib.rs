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

mod identifier;
mod interface;

#[derive(Debug)]
pub enum Definition<'a> {
    Interface(interface::Interface<'a>),
}

#[derive(Debug)]
pub struct Mojom<'a> {
    pub definitions: Vec<Definition<'a>>,
}

/// parses `input`.
pub fn parse(input: &str) -> Result<Mojom> {
    let input = Span::new(input.into());
    let parsed = interface::interface(input);

    parsed
        .map(|intr| {
            let definitions = vec![Definition::Interface(intr.1)];
            Mojom {
                definitions: definitions,
            }
        })
        .map_err(|err| match err {
            nom::Err::Error(nom::Context::Code(code_span, _failed_parser)) => {
                Error::SyntaxError(code_span)
            }
            _ => unimplemented!(),
        })
}
