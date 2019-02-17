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
    // Inteface may have an attribute list. Skip them for now.
    let pair = intr.next().unwrap();
    let pair = match pair.as_rule() {
        Rule::attribute_section => intr.next().unwrap(),
        _ => pair,
    };

    let name = pair.as_span();
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
    fn test_parse() {
        let input = "\n
        interface InterfaceA {};

        // This is comment.
        interface InterfaceB {};

        [Attr]
        interface InterfaceC {};
        ";
        let res = parse(input).unwrap();
        assert_eq!(3, res.definitions.len());
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

    fn parse_part(r: Rule, i: &str) -> &str {
        MojomParser::parse(r, i).unwrap().as_str()
    }

    #[test]
    fn test_integer() {
        assert_eq!("0", parse_part(Rule::integer, "0"));
        assert_eq!("123", parse_part(Rule::integer, "123"));
        assert_eq!("-42", parse_part(Rule::integer, "-42"));
        assert_eq!("0xdeadbeef", parse_part(Rule::integer, "0xdeadbeef"));
        assert_eq!("+0X1AB4", parse_part(Rule::integer, "+0X1AB4"));
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(r#""hello""#, parse_part(Rule::string_literal, r#""hello""#));
        assert_eq!(
            r#""hell\"o""#,
            parse_part(Rule::string_literal, r#""hell\"o""#)
        );
    }

    #[test]
    fn test_literal() {
        assert_eq!("true", parse_part(Rule::literal, "true"));
        assert_eq!("false", parse_part(Rule::literal, "false"));
        assert_eq!("default", parse_part(Rule::literal, "default"));
        assert_eq!("0x12ab", parse_part(Rule::literal, "0x12ab"));
        assert_eq!(
            r#""string literal \"with\" quote""#,
            parse_part(Rule::literal, r#""string literal \"with\" quote""#)
        );
    }

    #[test]
    fn test_attribute() {
        assert_eq!("[]", parse_part(Rule::attribute_section, "[]"));
        assert_eq!(
            "[Attr1, Attr2=NameVal, Attr3=123]",
            parse_part(Rule::attribute_section, "[Attr1, Attr2=NameVal, Attr3=123]")
        );
    }
}
