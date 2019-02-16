#[macro_use]
extern crate nom;
#[macro_use]
extern crate nom_locate;

use nom::types::CompleteStr;

/// Span represents a slice of input &str along with its position.
pub type Span<'a> = nom_locate::LocatedSpan<CompleteStr<'a>>;

#[derive(Debug)]
pub enum Error<'a> {
    SyntaxError(Span<'a>),
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

mod definition;
mod identifier;
mod interface;
mod mojom;

use mojom::{mojom_file, Mojom};

/// parses `input`.
pub fn parse(input: &str) -> Result<Mojom> {
    let input = Span::new(input.into());
    let (_next, ast) = mojom_file(input).map_err(|err| match err {
        nom::Err::Error(nom::Context::Code(code_span, _failed_parser)) => {
            Error::SyntaxError(code_span)
        }
        _ => unimplemented!(),
    })?;
    Ok(ast)
}
