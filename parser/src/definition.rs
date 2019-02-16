use crate::interface::{interface, Interface};

use crate::Span;

#[derive(Debug)]
pub enum Definition<'a> {
    Interface(Interface<'a>),
}

named!(pub definition<Span, Definition>,
    do_parse!(
        intr: interface >>
        (Definition::Interface(intr))
    )
);
