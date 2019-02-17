use pest::Parser;

use crate::{Error, Span};

#[derive(Parser)]
#[grammar = "mojom.pest"]
struct MojomParser;

#[derive(Debug)]
pub struct Interface<'a> {
    pub name: Span<'a>,
}

#[derive(Debug)]
pub enum Definition<'a> {
    Interface(Interface<'a>),
    _Dummy, // Just for enforcing multiple match arms
}

#[derive(Debug)]
pub struct Mojom<'a> {
    pub definitions: Vec<Definition<'a>>,
}

use pest::iterators::Pairs;

fn conv_interface<'a>(mut intr: Pairs<'a, Rule>) -> Interface<'a> {
    let name = intr.next().unwrap().as_span();
    Interface { name: name }
}

/// Parses `input`.
pub fn parse(input: &str) -> Result<Mojom, Error<Rule>> {
    // If parsing is successful, it safe to assume that next().unwrap() returns
    // valid `mojo_file` rule.
    let parsed = MojomParser::parse(Rule::mojom_file, &input)?
        .next()
        .unwrap()
        .into_inner();

    let mut definitions = Vec::new();
    for stmt in parsed {
        match stmt.as_rule() {
            Rule::interface => {
                let intr = conv_interface(stmt.into_inner());
                definitions.push(Definition::Interface(intr));
            }
            Rule::EOI => break,
            _ => unreachable!(),
        }
    }

    Ok(Mojom {
        definitions: definitions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mojom_file() {
        let input = "\n
        interface InterfaceA {};

        // This is comment.
        interface InterfaceB {};
        ";
        let res = parse(input).unwrap();
        assert_eq!(2, res.definitions.len());
    }

    #[test]
    fn test_comment() {
        let input = "/* block comment */";
        let parsed = MojomParser::parse(Rule::mojom_file, &input);
        assert!(parsed.is_ok());

        let input = "// line comment";
        let parsed = MojomParser::parse(Rule::mojom_file, &input);
        assert!(parsed.is_ok());
    }
}
