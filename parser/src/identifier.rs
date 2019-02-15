use nom::alphanumeric;

use crate::Span;

#[derive(Debug, PartialEq)]
pub struct Identifier<'a> {
    value: Span<'a>,
}

impl<'a> Identifier<'a> {
    pub fn value(&self) -> &str {
        self.value.fragment.as_ref()
    }
}

named!(pub identifier<Span, Identifier>,
    ws!(do_parse!(
        _pos: position!() >>
        value: alphanumeric >>
        (Identifier { value: value })))
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier() {
        let input = Span::new("\nhello world".into());
        let (next, res) = identifier(input).unwrap();
        assert_eq!("hello", res.value());
        assert_eq!("world", next.fragment.as_ref());
    }
}
