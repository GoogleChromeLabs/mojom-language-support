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
pub struct Module<'a> {
    pub name: Span<'a>,
}

fn into_module<'a>(mut pairs: Pairs<'a>) -> Module<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    Module { name: name }
}

#[derive(Debug)]
pub struct Import<'a> {
    pub path: Span<'a>,
}

fn into_import<'a>(mut pairs: Pairs<'a>) -> Import<'a> {
    let path = pairs.next().unwrap().as_span();
    Import { path: path }
}

#[derive(Debug)]
pub struct Const<'a> {
    pub typ: Span<'a>,
    pub name: Span<'a>,
    pub value: Span<'a>,
}

fn into_const<'a>(mut pairs: Pairs<'a>) -> Const<'a> {
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

#[derive(Debug)]
pub struct EnumValue<'a> {
    pub name: Span<'a>,
    pub value: Option<Span<'a>>,
}

fn into_enum_value<'a>(mut pairs: Pairs<'a>) -> EnumValue<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let value = pairs.next().map(|value| value.as_span());
    EnumValue {
        name: name,
        value: value,
    }
}

#[derive(Debug)]
pub struct Enum<'a> {
    pub name: Span<'a>,
    pub values: Vec<EnumValue<'a>>,
}

fn into_enum<'a>(mut pairs: Pairs<'a>) -> Enum<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let values = match pairs.next() {
        Some(pairs) => {
            let mut values = Vec::new();
            for item in pairs.into_inner() {
                values.push(into_enum_value(item.into_inner()));
            }
            values
        }
        None => Vec::new(),
    };
    Enum {
        name: name,
        values: values,
    }
}

#[derive(Debug)]
pub struct StructField<'a> {
    typ: Span<'a>,
    name: Span<'a>,
    ordinal: Option<Span<'a>>,
    default: Option<Span<'a>>,
}

fn into_struct_field<'a>(mut pairs: Pairs<'a>) -> StructField<'a> {
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
    pub members: Vec<StructBody<'a>>,
}

fn into_struct<'a>(mut pairs: Pairs<'a>) -> Struct<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let body = pairs.next();
    let members = match body {
        Some(pairs) => {
            let mut members = Vec::new();
            for inner in pairs.into_inner() {
                let item = inner.into_inner().next().unwrap();
                let item = match item.as_rule() {
                    Rule::const_stmt => StructBody::Const(into_const(item.into_inner())),
                    Rule::enum_stmt => StructBody::Enum(into_enum(item.into_inner())),
                    Rule::struct_field => StructBody::Field(into_struct_field(item.into_inner())),
                    _ => unreachable!(),
                };
                members.push(item);
            }
            members
        }
        None => Vec::new(),
    };
    Struct {
        name: name,
        members: members,
    }
}

#[derive(Debug)]
pub struct UnionField<'a> {
    pub typ: Span<'a>,
    pub name: Span<'a>,
    pub ordinal: Option<Span<'a>>,
}

fn into_union_field<'a>(mut pairs: Pairs<'a>) -> UnionField<'a> {
    _skip_attribute_list(&mut pairs);
    let typ = pairs.next().unwrap().as_span();
    let name = pairs.next().unwrap().as_span();
    let ordinal = pairs.next().map(|ord| ord.as_span());
    UnionField {
        typ: typ,
        name: name,
        ordinal: ordinal,
    }
}

#[derive(Debug)]
pub struct Union<'a> {
    pub name: Span<'a>,
    pub fields: Vec<UnionField<'a>>,
}

fn into_union<'a>(mut pairs: Pairs<'a>) -> Union<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let mut fields = Vec::new();
    for inner in pairs {
        let item = inner;
        let item = match item.as_rule() {
            Rule::union_field => into_union_field(item.into_inner()),
            _ => unreachable!(),
        };
        fields.push(item);
    }
    Union {
        name: name,
        fields: fields,
    }
}

#[derive(Debug)]
pub struct Parameter<'a> {
    pub typ: Span<'a>,
    pub name: Span<'a>,
    pub ordinal: Option<Span<'a>>,
}

fn into_parameter<'a>(mut pairs: Pairs<'a>) -> Parameter<'a> {
    let typ = pairs.next().unwrap().as_span();
    let name = pairs.next().unwrap().as_span();
    let ordinal = pairs.next().map(|ord| ord.as_span());
    Parameter {
        typ: typ,
        name: name,
        ordinal: ordinal,
    }
}

fn _parameter_list<'a>(pairs: Pairs<'a>) -> Vec<Parameter<'a>> {
    pairs
        .map(|param| into_parameter(param.into_inner()))
        .collect::<Vec<_>>()
}

#[derive(Debug)]
pub struct Response<'a> {
    pub params: Vec<Parameter<'a>>,
}

fn into_response<'a>(mut pairs: Pairs<'a>) -> Response<'a> {
    let params = _parameter_list(pairs.next().unwrap().into_inner());
    Response { params: params }
}

#[derive(Debug)]
pub struct Method<'a> {
    pub name: Span<'a>,
    pub ordinal: Option<Span<'a>>,
    pub params: Vec<Parameter<'a>>,
    pub response: Option<Response<'a>>,
}

fn into_method<'a>(mut pairs: Pairs<'a>) -> Method<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let ordinal = match pairs.peek().unwrap().as_rule() {
        Rule::ordinal_value => pairs.next().map(|ord| ord.as_span()),
        _ => None,
    };
    let params = _parameter_list(pairs.next().unwrap().into_inner());
    let response = pairs.next().map(|res| into_response(res.into_inner()));
    Method {
        name: name,
        ordinal: ordinal,
        params: params,
        response: response,
    }
}

#[derive(Debug)]
pub enum InterfaceMember<'a> {
    Const(Const<'a>),
    Enum(Enum<'a>),
    Method(Method<'a>),
}

#[derive(Debug)]
pub struct Interface<'a> {
    pub name: Span<'a>,
    pub members: Vec<InterfaceMember<'a>>,
}

fn into_interface<'a>(mut pairs: Pairs<'a>) -> Interface<'a> {
    _skip_attribute_list(&mut pairs);
    let name = pairs.next().unwrap().as_span();
    let mut members = Vec::new();
    for member in pairs {
        let member = member.into_inner().next().unwrap();
        let member = match member.as_rule() {
            Rule::const_stmt => InterfaceMember::Const(into_const(member.into_inner())),
            Rule::enum_stmt => InterfaceMember::Enum(into_enum(member.into_inner())),
            Rule::method_stmt => InterfaceMember::Method(into_method(member.into_inner())),
            _ => unreachable!(),
        };
        members.push(member);
    }
    Interface {
        name: name,
        members: members,
    }
}

#[derive(Debug)]
pub enum Statement<'a> {
    Module(Module<'a>),
    Import(Import<'a>),
    Interface(Interface<'a>),
    Struct(Struct<'a>),
    Union(Union<'a>),
    Enum(Enum<'a>),
    Const(Const<'a>),
}

fn into_statement<'a>(mut pairs: Pairs<'a>) -> Statement<'a> {
    let stmt = pairs.next().unwrap();
    match stmt.as_rule() {
        Rule::module_stmt => Statement::Module(into_module(stmt.into_inner())),
        Rule::import_stmt => Statement::Import(into_import(stmt.into_inner())),
        Rule::interface => Statement::Interface(into_interface(stmt.into_inner())),
        Rule::struct_stmt => Statement::Struct(into_struct(stmt.into_inner())),
        Rule::union_stmt => Statement::Union(into_union(stmt.into_inner())),
        Rule::enum_stmt => Statement::Enum(into_enum(stmt.into_inner())),
        Rule::const_stmt => Statement::Const(into_const(stmt.into_inner())),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct MojomFile<'a> {
    pub stmts: Vec<Statement<'a>>,
}

fn into_mojom_file<'a>(pairs: Pairs<'a>) -> MojomFile<'a> {
    let mut stmts = Vec::new();
    for stmt in pairs {
        let stmt = match stmt.as_rule() {
            Rule::statement => into_statement(stmt.into_inner()),
            Rule::EOI => break,
            _ => unreachable!(),
        };
        stmts.push(stmt);
    }
    MojomFile { stmts: stmts }
}

/// Parses `input`.
pub fn parse(input: &str) -> Result<MojomFile, Error<Rule>> {
    let parsed = MojomParser::parse(Rule::mojom_file, &input)?
        .next()
        .unwrap()
        .into_inner();
    let mojom = into_mojom_file(parsed);
    Ok(mojom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let input = r#"
        module test.module;
        import "a.b.c";
        import "a.c.d";

        enum MyEnum { kFoo, kBar, kBaz };
        const string kMyConst = "const_value";

        // This is MyInterface
        interface MyInterface {
            MyMethod() => (/* empty */);
        };

        interface InterfaceA {};

        // This is comment.
        interface InterfaceB {};

        [Attr]
        interface InterfaceC {};

        interface InterfaceD {
            const string kMessage = "message";
            enum SomeEnum { Foo, Bar, Baz, };
            MethodA(string message) => ();
            MethodB() => (int32 result);
            [Attr2] MethodC(associated InterfaceA assoc) => (map<string, int8> result);
        };
        "#;
        let res = parse(input).unwrap();
        assert_eq!(10, res.stmts.len());
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
    fn test_float() {
        assert_eq!("0.0", parse_part(Rule::float, "0.0"));
        assert_eq!("1.0", parse_part(Rule::float, "1.0"));
        assert_eq!("3.141", parse_part(Rule::float, "3.141"));
        assert_eq!("+0.123", parse_part(Rule::float, "+0.123"));
        assert_eq!("-5.67", parse_part(Rule::float, "-5.67"));
        assert_eq!("4e5", parse_part(Rule::float, "4e5"));
        assert_eq!("-7e+15", parse_part(Rule::float, "-7e+15"));
        assert_eq!("+9e-2", parse_part(Rule::float, "+9e-2"));
    }

    #[test]
    fn test_number() {
        assert_eq!("0", parse_part(Rule::number, "0"));
        assert_eq!("123", parse_part(Rule::number, "123"));
        assert_eq!("-42", parse_part(Rule::number, "-42"));
        assert_eq!("0xdeadbeef", parse_part(Rule::number, "0xdeadbeef"));
        assert_eq!("+0X1AB4", parse_part(Rule::number, "+0X1AB4"));

        assert_eq!("0.0", parse_part(Rule::number, "0.0"));
        assert_eq!("1.0", parse_part(Rule::number, "1.0"));
        assert_eq!("3.141", parse_part(Rule::number, "3.141"));
        assert_eq!("+0.123", parse_part(Rule::number, "+0.123"));
        assert_eq!("-5.67", parse_part(Rule::number, "-5.67"));
        assert_eq!("4e5", parse_part(Rule::number, "4e5"));
        assert_eq!("-7e+15", parse_part(Rule::number, "-7e+15"));
        assert_eq!("+9e-2", parse_part(Rule::number, "+9e-2"));
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
    fn test_module_stmt() {
        let input = "module my.mod;";
        let parsed = MojomParser::parse(Rule::module_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_module(parsed.into_inner());
        assert_eq!("my.mod", stmt.name.as_str());
    }

    #[test]
    fn test_import_stmt() {
        let input = r#"import "my.mod";"#;
        let parsed = MojomParser::parse(Rule::import_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_import(parsed.into_inner());
        assert_eq!(r#""my.mod""#, stmt.path.as_str());
    }

    #[test]
    fn test_const_stmt() {
        let input = "const uint32 kTheAnswer = 42;";
        let parsed = MojomParser::parse(Rule::const_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_const(parsed.into_inner());
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
        let stmt = into_enum(parsed.into_inner());
        assert_eq!("MyEnum", stmt.name.as_str());
        let values = &stmt.values;
        assert_eq!(3, values.len());
        assert_eq!("kOne", values[0].name.as_str());
        assert_eq!("kTwo", values[1].name.as_str());
        assert_eq!("2", values[1].value.as_ref().unwrap().as_str());
        assert_eq!("kThree", values[2].name.as_str());
        assert_eq!("IdentValue", values[2].value.as_ref().unwrap().as_str());

        let input = "[Native] enum MyEnum;";
        let parsed = MojomParser::parse(Rule::enum_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_enum(parsed.into_inner());
        assert_eq!("MyEnum", stmt.name.as_str());
        assert_eq!(0, stmt.values.len());
    }

    #[test]
    fn test_method_stmt() {
        let input = "MyMethod(string str_arg, int8 int8_arg) => (uint32 result);";
        let parsed = MojomParser::parse(Rule::method_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_method(parsed.into_inner());
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
        let stmt = into_method(parsed.into_inner());
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
            float my_float_value = 0.1;
        };";
        let parsed = MojomParser::parse(Rule::struct_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_struct(parsed.into_inner());
        assert_eq!("MyStruct", stmt.name.as_str());
        let members = &stmt.members;
        assert_eq!(4, members.len());

        let item = match &members[0] {
            StructBody::Const(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("kInvalidId", item.name.as_str());

        let item = match &members[1] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_id", item.name.as_str());

        let item = match &members[2] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_interface", item.name.as_str());

        let item = match &members[3] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_float_value", item.name.as_str());

        let input = "[Native] struct MyStruct;";
        let parsed = MojomParser::parse(Rule::struct_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_struct(parsed.into_inner());
        assert_eq!("MyStruct", stmt.name.as_str());
        assert_eq!(0, stmt.members.len());
    }

    #[test]
    fn test_interface() {
        let input = "interface MyInterface {
            MyMethod();
            enum MyEnum { kMyEnumVal1, kMyEnumVal2 };
        };";
        let parsed = MojomParser::parse(Rule::interface, &input)
            .unwrap()
            .next()
            .unwrap();
        let intr = into_interface(parsed.into_inner());
        assert_eq!("MyInterface", intr.name.as_str());
        let members = &intr.members;
        assert_eq!(2, members.len());

        let member = match &members[0] {
            InterfaceMember::Method(member) => member,
            _ => unreachable!(),
        };
        assert_eq!("MyMethod", member.name.as_str());

        let member = match &members[1] {
            InterfaceMember::Enum(member) => member,
            _ => unreachable!(),
        };
        assert_eq!("MyEnum", member.name.as_str());
    }

    #[test]
    fn test_union_stmt() {
        let input = "union MyUnion {
            string str_field;
            StringPair pair_field;
            int64 int64_field;
        };";
        let parsed = MojomParser::parse(Rule::union_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_union(parsed.into_inner());
        assert_eq!("MyUnion", stmt.name.as_str());
        let fields = &stmt.fields;
        assert_eq!(3, fields.len());
        assert_eq!("str_field", fields[0].name.as_str());
        assert_eq!("pair_field", fields[1].name.as_str());
        assert_eq!("int64_field", fields[2].name.as_str());
    }
}
