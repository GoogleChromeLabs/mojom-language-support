use pest::Parser;

use crate::{Error, Span};

#[derive(Parser)]
#[grammar = "mojom.pest"]
struct MojomParser;

type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

// Skips attribute list if exists. This is tentative.
fn _skip_attribute_list(pairs: &mut Pairs) {
    match pairs.peek().unwrap().as_rule() {
        Rule::attribute_section => {
            pairs.next();
        }
        _ => (),
    }
}

#[derive(Debug)]
pub struct Const<'a> {
    pub typ: Span<'a>,
    pub name: Span<'a>,
    pub value: Span<'a>,
}

impl<'a> From<Pairs<'a>> for Const<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        _skip_attribute_list(&mut pairs);
        let pair = pairs.next().unwrap();
        let typ = pair.as_span();
        let name = pairs.next().unwrap().as_span();
        let value = pairs.next().unwrap().as_span();
        Const {
            typ: typ,
            name: name,
            value: value,
        }
    }
}

#[derive(Debug)]
pub struct EnumValue<'a> {
    pub name: Span<'a>,
    pub value: Option<Span<'a>>,
}

impl<'a> From<Pairs<'a>> for EnumValue<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        _skip_attribute_list(&mut pairs);
        let name = pairs.next().unwrap().as_span();
        let value = pairs.next().map(|value| value.as_span());
        EnumValue {
            name: name,
            value: value,
        }
    }
}

#[derive(Debug)]
pub struct Enum<'a> {
    pub name: Span<'a>,
    pub values: Vec<EnumValue<'a>>,
}

impl<'a> From<Pairs<'a>> for Enum<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        _skip_attribute_list(&mut pairs);
        let name = pairs.next().unwrap().as_span();
        let mut values = Vec::new();
        for item in pairs {
            values.push(EnumValue::from(item.into_inner()));
        }
        Enum {
            name: name,
            values: values,
        }
    }
}

#[derive(Debug)]
pub struct StructField<'a> {
    typ: Span<'a>,
    name: Span<'a>,
    ordinal: Option<Span<'a>>,
    default: Option<Span<'a>>,
}

fn _conv_struct_field<'a>(mut pairs: Pairs<'a>) -> StructField<'a> {
    _skip_attribute_list(&mut pairs);
    let typ = pairs.next().unwrap().as_span();
    let name = pairs.next().unwrap().as_span();
    let mut res = StructField {
        typ: typ,
        name: name,
        ordinal: None,
        default: None,
    };
    for pair in pairs {
        match pair.as_rule() {
            Rule::ordinal_value => res.ordinal = Some(pair.as_span()),
            Rule::default => res.default = Some(pair.as_span()),
            _ => unreachable!(),
        }
    }
    res
}

#[derive(Debug)]
pub enum StructBody<'a> {
    Const(Const<'a>),
    Enum(Enum<'a>),
    Field(StructField<'a>),
}

#[derive(Debug)]
pub struct Struct<'a> {
    pub name: Span<'a>,
    pub body: Vec<StructBody<'a>>,
}

fn _conv_struct<'a>(mut pairs: Pairs<'a>) -> Struct<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let mut body = Vec::new();
    for inner in pairs.next().unwrap().into_inner() {
        let item = inner.into_inner().next().unwrap();
        let item = match item.as_rule() {
            Rule::const_stmt => StructBody::Const(Const::from(item.into_inner())),
            Rule::enum_stmt => StructBody::Enum(Enum::from(item.into_inner())),
            Rule::struct_field => StructBody::Field(_conv_struct_field(item.into_inner())),
            _ => {
                println!("{:?}", item);
                unreachable!()
            }
        };
        body.push(item);
    }
    Struct {
        name: name,
        body: body,
    }
}

#[derive(Debug)]
pub struct Parameter<'a> {
    pub typ: Span<'a>,
    pub name: Span<'a>,
    pub ordinal: Option<Span<'a>>,
}

impl<'a> From<Pairs<'a>> for Parameter<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        let typ = pairs.next().unwrap().as_span();
        let name = pairs.next().unwrap().as_span();
        let ordinal = pairs.next().map(|ord| ord.as_span());
        Parameter {
            typ: typ,
            name: name,
            ordinal: ordinal,
        }
    }
}

fn _parameter_list<'a>(pairs: Pairs<'a>) -> Vec<Parameter<'a>> {
    pairs
        .map(|param| Parameter::from(param.into_inner()))
        .collect::<Vec<_>>()
}

#[derive(Debug)]
pub struct Response<'a> {
    pub params: Vec<Parameter<'a>>,
}

impl<'a> From<Pairs<'a>> for Response<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        let params = _parameter_list(pairs.next().unwrap().into_inner());
        Response { params: params }
    }
}

#[derive(Debug)]
pub struct Method<'a> {
    pub name: Span<'a>,
    pub ordinal: Option<Span<'a>>,
    pub params: Vec<Parameter<'a>>,
    pub response: Option<Response<'a>>,
}

impl<'a> From<Pairs<'a>> for Method<'a> {
    fn from(mut pairs: Pairs<'a>) -> Self {
        _skip_attribute_list(&mut pairs);
        let name = pairs.next().unwrap().as_span();
        let ordinal = match pairs.peek().unwrap().as_rule() {
            Rule::ordinal_value => pairs.next().map(|ord| ord.as_span()),
            _ => None,
        };
        let params = _parameter_list(pairs.next().unwrap().into_inner());
        let response = pairs.next().map(|res| Response::from(res.into_inner()));
        Method {
            name: name,
            ordinal: ordinal,
            params: params,
            response: response,
        }
    }
}

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

fn conv_interface<'a>(mut intr: Pairs<'a>) -> Interface<'a> {
    _skip_attribute_list(&mut intr);
    let pair = intr.next().unwrap();
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

        interface InterfaceD {
            const string kMessage = \"message\";
            enum SomeEnum { Foo, Bar, Baz, };
            MethodA(string message) => ();
            MethodB() => (int32 result);
            [Attr2] MethodC(associated InterfaceA assoc) => (map<string, int8> result);
        };
        ";
        let res = parse(input).unwrap();
        assert_eq!(4, res.definitions.len());
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

    #[test]
    fn test_types() {
        macro_rules! parse_type {
            ($tok:expr) => {{
                assert_eq!($tok, parse_part(Rule::type_spec, $tok));
            }};
        }

        parse_type!("bool");
        parse_type!("int8");
        parse_type!("uint8");
        parse_type!("int16");
        parse_type!("uint16");
        parse_type!("int32");
        parse_type!("uint32");
        parse_type!("int64");
        parse_type!("uint64");
        parse_type!("float");
        parse_type!("double");
        parse_type!("handle");
        parse_type!("handle<message_pipe>");
        parse_type!("string");
        parse_type!("array<uint8>");
        parse_type!("array<uint8, 16>");
        parse_type!("map<int32, MyInterface>");
        parse_type!("MyInterface");
        parse_type!("MyInerface&");
        parse_type!("associated MyInterface");
        parse_type!("associated MyInterface&");
        parse_type!("bool?");
    }

    #[test]
    fn test_const_stmt() {
        let input = "const uint32 kTheAnswer = 42;";
        let parsed = MojomParser::parse(Rule::const_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = Const::from(parsed.into_inner());
        assert_eq!("uint32", stmt.typ.as_str());
        assert_eq!("kTheAnswer", stmt.name.as_str());
        assert_eq!("42", stmt.value.as_str());
    }

    #[test]
    fn test_enum_stmt() {
        let input = "enum MyEnum { kOne, kTwo=2, kThree=IdentValue, };";
        let parsed = MojomParser::parse(Rule::enum_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = Enum::from(parsed.into_inner());
        assert_eq!("MyEnum", stmt.name.as_str());
        let values = &stmt.values;
        assert_eq!(3, values.len());
        assert_eq!("kOne", values[0].name.as_str());
        assert_eq!("kTwo", values[1].name.as_str());
        assert_eq!("2", values[1].value.as_ref().unwrap().as_str());
        assert_eq!("kThree", values[2].name.as_str());
        assert_eq!("IdentValue", values[2].value.as_ref().unwrap().as_str());
    }

    #[test]
    fn test_method_stmt() {
        let input = "MyMethod(string str_arg, int8 int8_arg) => (uint32 result);";
        let parsed = MojomParser::parse(Rule::method_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = Method::from(parsed.into_inner());
        assert_eq!("MyMethod", stmt.name.as_str());
        let params = &stmt.params;
        assert_eq!(2, params.len());
        assert_eq!("string", params[0].typ.as_str());
        assert_eq!("str_arg", params[0].name.as_str());
        assert_eq!("int8", params[1].typ.as_str());
        assert_eq!("int8_arg", params[1].name.as_str());
        let response = stmt.response.as_ref().unwrap();
        assert_eq!(1, response.params.len());
        assert_eq!("uint32", response.params[0].typ.as_str());
        assert_eq!("result", response.params[0].name.as_str());

        let input = "MyMethod2();";
        let parsed = MojomParser::parse(Rule::method_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = Method::from(parsed.into_inner());
        assert_eq!("MyMethod2", stmt.name.as_str());
        assert_eq!(0, stmt.params.len());
        assert!(stmt.response.is_none());
    }

    #[test]
    fn test_struct_stmt() {
        let input = "struct MyStruct {
            const int64 kInvalidId = -1;
            int64 my_id;
            MyInterface? my_interface;
        };";
        let parsed = MojomParser::parse(Rule::struct_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = _conv_struct(parsed.into_inner());
        assert_eq!("MyStruct", stmt.name.as_str());
        let body = &stmt.body;
        assert_eq!(3, body.len());

        let item = match &body[0] {
            StructBody::Const(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("kInvalidId", item.name.as_str());

        let item = match &body[1] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_id", item.name.as_str());

        let item = match &body[2] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_interface", item.name.as_str());
    }
}
