use crate::definition::{definition, Definition};

use crate::Span;

#[derive(Debug)]
pub struct Mojom<'a> {
    pub definitions: Vec<Definition<'a>>,
}

named!(pub mojom_file<Span, Mojom>, do_parse!(
    defs: many0!(definition) >>
    (Mojom {
        definitions: defs,
    })
));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mojom_file() {
        let input = Span::new(
            "\n
        interface InterfaceA {};
        
        interface InterfaceB {};
        "
            .into(),
        );
        let res = mojom_file(input).unwrap().1;
        assert_eq!(2, res.definitions.len());

        let input = Span::new("interface InterfaceA { a };".into());
        let res = mojom_file(input);
        println!("{:?}", input);
    }
}
