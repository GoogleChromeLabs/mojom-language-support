use crate::identifier::{identifier, Identifier};

use crate::Span;

#[derive(Debug)]
pub struct Interface<'a> {
    pub name: Identifier<'a>,
}

named!(pub interface<Span, Interface>,
    do_parse!(
        ws!(tag!("interface")) >>
        name: identifier >>
        ws!(tag!("{")) >>
        // TODO: Parse body
        ws!(tag!("}")) >>
        tag!(";") >>
        (Interface {
            name: name,
        })
    )
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface() {
        let input = Span::new("interface MyInterface {};".into());
        let res = interface(input).unwrap().1;
        assert_eq!("MyInterface", res.name.value());
    }
}
