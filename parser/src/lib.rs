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

pub struct Mojom<'a> {
    pub definitions: Vec<Definition<'a>>,
}

/// parses `input`.
pub fn parse(input: &str) -> Result<Mojom, nom::Err<Span>> {
    let input = Span::new(input.into());
    interface::interface(input).map(|intr| {
        let intr = intr.1;
        let definitions = vec![Definition::Interface(intr)];
        Mojom {
            definitions: definitions,
        }
    })
}
