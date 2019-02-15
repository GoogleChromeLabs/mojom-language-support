use crate::interface;

use crate::{Error, Result, Span};

#[derive(Debug)]
pub enum Definition<'a> {
    Interface(interface::Interface<'a>),
}

pub fn definitions(input: Span) -> Result<Vec<Definition>> {
    let parsed = interface::interface(input);
    // eprintln!("{:?}", parsed);

    parsed
        .map(|intr| {
            let definitions = vec![Definition::Interface(intr.1)];
            definitions
        })
        .map_err(|err| match err {
            nom::Err::Error(nom::Context::Code(code_span, _failed_parser)) => {
                Error::SyntaxError(code_span)
            }
            _ => unimplemented!(),
        })
}
