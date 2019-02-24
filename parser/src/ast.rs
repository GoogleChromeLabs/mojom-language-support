use pest::{Parser, Position, Span};

use crate::parser::{MojomParser, Rule};
use crate::Error;

type Pairs<'a> = pest::iterators::Pairs<'a, Rule>;

#[derive(Debug)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl<'a> From<Span<'a>> for Range {
    fn from(span: Span<'a>) -> Range {
        Range {
            start: span.start(),
            end: span.end(),
        }
    }
}

// Skips attribute list if exists. This is tentative.
fn skip_attribute_list(pairs: &mut Pairs) {
    match pairs.peek().unwrap().as_rule() {
        Rule::attribute_section => {
            pairs.next();
        }
        _ => (),
    }
}

fn consume_semicolon(pairs: &mut Pairs) {
    match pairs.next().unwrap().as_rule() {
        Rule::t_semicolon => (),
        _ => unreachable!(),
    };
}

fn consume_as_range(pairs: &mut Pairs) -> Range {
    pairs.next().unwrap().as_span().into()
}

#[derive(Debug)]
pub struct Module {
    pub name: Range,
}

fn into_module(mut pairs: Pairs) -> Module {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    consume_semicolon(&mut pairs);
    Module { name: name }
}

#[derive(Debug)]
pub struct Import {
    pub path: Range,
}

fn into_import(mut pairs: Pairs) -> Import {
    skip_attribute_list(&mut pairs);
    let path = consume_as_range(&mut pairs);
    consume_semicolon(&mut pairs);
    Import { path: path }
}

#[derive(Debug)]
pub struct Const {
    pub typ: Range,
    pub name: Range,
    pub value: Range,
}

fn into_const(mut pairs: Pairs) -> Const {
    skip_attribute_list(&mut pairs);
    let pair = pairs.next().unwrap();
    let typ = pair.as_span().into();
    let name = consume_as_range(&mut pairs);
    let value = consume_as_range(&mut pairs);
    consume_semicolon(&mut pairs);
    Const {
        typ: typ,
        name: name,
        value: value,
    }
}

#[derive(Debug)]
pub struct EnumValue {
    pub name: Range,
    pub value: Option<Range>,
}

fn into_enum_value(mut pairs: Pairs) -> EnumValue {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let value = pairs.next().map(|value| value.as_span().into());
    EnumValue {
        name: name,
        value: value,
    }
}

#[derive(Debug)]
pub struct Enum {
    pub name: Range,
    pub values: Vec<EnumValue>,
}

fn into_enum(mut pairs: Pairs) -> Enum {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let mut values = Vec::new();
    for item in pairs {
        match item.as_rule() {
            Rule::enum_block => {
                for item in item.into_inner() {
                    values.push(into_enum_value(item.into_inner()));
                }
            }
            Rule::t_semicolon => break,
            _ => unreachable!(),
        }
    }
    Enum {
        name: name,
        values: values,
    }
}

#[derive(Debug)]
pub struct StructField {
    typ: Range,
    name: Range,
    ordinal: Option<Range>,
    default: Option<Range>,
}

fn into_struct_field(mut pairs: Pairs) -> StructField {
    skip_attribute_list(&mut pairs);
    let typ = consume_as_range(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let mut res = StructField {
        typ: typ,
        name: name,
        ordinal: None,
        default: None,
    };
    for item in pairs {
        match item.as_rule() {
            Rule::ordinal_value => res.ordinal = Some(item.as_span().into()),
            Rule::default => res.default = Some(item.as_span().into()),
            Rule::t_semicolon => break,
            _ => unreachable!(),
        }
    }
    res
}

#[derive(Debug)]
pub enum StructBody {
    Const(Const),
    Enum(Enum),
    Field(StructField),
}

#[derive(Debug)]
pub struct Struct {
    pub name: Range,
    pub members: Vec<StructBody>,
}

fn into_struct(mut pairs: Pairs) -> Struct {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
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
                    Rule::t_semicolon => break,
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
pub struct UnionField {
    pub typ: Range,
    pub name: Range,
    pub ordinal: Option<Range>,
}

fn into_union_field(mut pairs: Pairs) -> UnionField {
    skip_attribute_list(&mut pairs);
    let typ = consume_as_range(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let mut ordinal = None;
    for item in pairs {
        match item.as_rule() {
            Rule::ordinal_value => ordinal = Some(item.as_span().into()),
            Rule::t_semicolon => break,
            _ => unreachable!(),
        }
    }
    UnionField {
        typ: typ,
        name: name,
        ordinal: ordinal,
    }
}

#[derive(Debug)]
pub struct Union {
    pub name: Range,
    pub fields: Vec<UnionField>,
}

fn into_union(mut pairs: Pairs) -> Union {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let mut fields = Vec::new();
    for item in pairs {
        let item = match item.as_rule() {
            Rule::union_field => into_union_field(item.into_inner()),
            Rule::t_semicolon => break,
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
pub struct Parameter {
    pub typ: Range,
    pub name: Range,
    pub ordinal: Option<Range>,
}

fn into_parameter(mut pairs: Pairs) -> Parameter {
    let typ = consume_as_range(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let ordinal = pairs.next().map(|ord| ord.as_span().into());
    Parameter {
        typ: typ,
        name: name,
        ordinal: ordinal,
    }
}

fn _parameter_list(pairs: Pairs) -> Vec<Parameter> {
    pairs
        .map(|param| into_parameter(param.into_inner()))
        .collect::<Vec<_>>()
}

#[derive(Debug)]
pub struct Response {
    pub params: Vec<Parameter>,
}

fn into_response(mut pairs: Pairs) -> Response {
    let params = _parameter_list(pairs.next().unwrap().into_inner());
    Response { params: params }
}

#[derive(Debug)]
pub struct Method {
    pub name: Range,
    pub ordinal: Option<Range>,
    pub params: Vec<Parameter>,
    pub response: Option<Response>,
}

fn into_method(mut pairs: Pairs) -> Method {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let ordinal = match pairs.peek().unwrap().as_rule() {
        Rule::ordinal_value => pairs.next().map(|ord| ord.as_span().into()),
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
pub enum InterfaceMember {
    Const(Const),
    Enum(Enum),
    Method(Method),
}

fn into_interface_member(mut pairs: Pairs) -> InterfaceMember {
    let member = pairs.next().unwrap();
    match member.as_rule() {
        Rule::const_stmt => InterfaceMember::Const(into_const(member.into_inner())),
        Rule::enum_stmt => InterfaceMember::Enum(into_enum(member.into_inner())),
        Rule::method_stmt => InterfaceMember::Method(into_method(member.into_inner())),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct Interface {
    pub name: Range,
    pub members: Vec<InterfaceMember>,
}

fn into_interface(mut pairs: Pairs) -> Interface {
    skip_attribute_list(&mut pairs);
    let name = consume_as_range(&mut pairs);
    let mut members = Vec::new();
    for item in pairs {
        match item.as_rule() {
            Rule::interface_body => {
                let member = into_interface_member(item.into_inner());
                members.push(member);
            }
            Rule::t_semicolon => break,
            _ => unreachable!(),
        }
    }
    Interface {
        name: name,
        members: members,
    }
}

#[derive(Debug)]
pub enum Statement {
    Module(Module),
    Import(Import),
    Interface(Interface),
    Struct(Struct),
    Union(Union),
    Enum(Enum),
    Const(Const),
}

fn into_statement(mut pairs: Pairs) -> Statement {
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
pub struct MojomFile {
    pub stmts: Vec<Statement>,
}

fn into_mojom_file(pairs: Pairs) -> MojomFile {
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

pub fn parse(input: &str) -> Result<MojomFile, Error<Rule>> {
    let parsed = MojomParser::parse(Rule::mojom_file, &input)?
        .next()
        .unwrap()
        .into_inner();
    let mojom = into_mojom_file(parsed);
    Ok(mojom)
}

#[derive(Debug)]
pub struct MojomAst {
    pub text: String,
    pub mojom: MojomFile,
}

impl MojomAst {
    pub fn new<S: Into<String>>(text: S) -> Result<MojomAst, Error<Rule>> {
        let text = text.into();
        let mojom = parse(&text)?;
        Ok(MojomAst {
            text: text,
            mojom: mojom,
        })
    }

    pub fn text(&self, field: &Range) -> &str {
        // Can panic.
        &self.text[field.start..field.end]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        // Can panic.
        let pos = Position::new(&self.text, offset).unwrap();
        pos.line_col()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partial_text<'t>(text: &'t str, range: &Range) -> &'t str {
        &text[range.start..range.end]
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
        assert_eq!("my.mod", partial_text(&input, &stmt.name));
    }

    #[test]
    fn test_import_stmt() {
        let input = r#"import "my.mod";"#;
        let parsed = MojomParser::parse(Rule::import_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_import(parsed.into_inner());
        assert_eq!(r#""my.mod""#, partial_text(&input, &stmt.path));

        let input = r#"[Attr] import "my.mod";"#;
        let parsed = MojomParser::parse(Rule::import_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_import(parsed.into_inner());
        assert_eq!(r#""my.mod""#, partial_text(&input, &stmt.path));
    }

    #[test]
    fn test_const_stmt() {
        let input = "const uint32 kTheAnswer = 42;";
        let parsed = MojomParser::parse(Rule::const_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_const(parsed.into_inner());
        assert_eq!("uint32", partial_text(&input, &stmt.typ));
        assert_eq!("kTheAnswer", partial_text(&input, &stmt.name));
        assert_eq!("42", partial_text(&input, &stmt.value));
    }

    #[test]
    fn test_enum_stmt() {
        let input = "enum MyEnum { kOne, kTwo=2, kThree=IdentValue, };";
        let parsed = MojomParser::parse(Rule::enum_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_enum(parsed.into_inner());
        assert_eq!("MyEnum", partial_text(&input, &stmt.name));
        let values = &stmt.values;
        assert_eq!(3, values.len());
        assert_eq!("kOne", partial_text(&input, &values[0].name));
        assert_eq!("kTwo", partial_text(&input, &values[1].name));
        assert_eq!("2", partial_text(&input, values[1].value.as_ref().unwrap()));
        assert_eq!("kThree", partial_text(&input, &values[2].name));
        assert_eq!(
            "IdentValue",
            partial_text(&input, values[2].value.as_ref().unwrap())
        );

        let input = "enum MyEnum {};";
        let parsed = MojomParser::parse(Rule::enum_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_enum(parsed.into_inner());
        assert_eq!("MyEnum", partial_text(&input, &stmt.name));
        assert_eq!(0, stmt.values.len());

        let input = "[Native] enum MyEnum;";
        let parsed = MojomParser::parse(Rule::enum_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_enum(parsed.into_inner());
        assert_eq!("MyEnum", partial_text(&input, &stmt.name));
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
        assert_eq!("MyMethod", partial_text(&input, &stmt.name));
        let params = &stmt.params;
        assert_eq!(2, params.len());
        assert_eq!("string", partial_text(&input, &params[0].typ));
        assert_eq!("str_arg", partial_text(&input, &params[0].name));
        assert_eq!("int8", partial_text(&input, &params[1].typ));
        assert_eq!("int8_arg", partial_text(&input, &params[1].name));
        let response = stmt.response.as_ref().unwrap();
        assert_eq!(1, response.params.len());
        assert_eq!("uint32", partial_text(&input, &response.params[0].typ));
        assert_eq!("result", partial_text(&input, &response.params[0].name));

        let input = "MyMethod2();";
        let parsed = MojomParser::parse(Rule::method_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_method(parsed.into_inner());
        assert_eq!("MyMethod2", partial_text(&input, &stmt.name));
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
        assert_eq!("MyStruct", partial_text(&input, &stmt.name));
        let members = &stmt.members;
        assert_eq!(4, members.len());

        let item = match &members[0] {
            StructBody::Const(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("kInvalidId", partial_text(&input, &item.name));

        let item = match &members[1] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_id", partial_text(&input, &item.name));

        let item = match &members[2] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_interface", partial_text(&input, &item.name));

        let item = match &members[3] {
            StructBody::Field(item) => item,
            _ => unreachable!(),
        };
        assert_eq!("my_float_value", partial_text(&input, &item.name));

        let input = "[Native] struct MyStruct;";
        let parsed = MojomParser::parse(Rule::struct_stmt, &input)
            .unwrap()
            .next()
            .unwrap();
        let stmt = into_struct(parsed.into_inner());
        assert_eq!("MyStruct", partial_text(&input, &stmt.name));
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
        assert_eq!("MyInterface", partial_text(&input, &intr.name));
        let members = &intr.members;
        assert_eq!(2, members.len());

        let member = match &members[0] {
            InterfaceMember::Method(member) => member,
            _ => unreachable!(),
        };
        assert_eq!("MyMethod", partial_text(&input, &member.name));

        let member = match &members[1] {
            InterfaceMember::Enum(member) => member,
            _ => unreachable!(),
        };
        assert_eq!("MyEnum", partial_text(&input, &member.name));
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
        assert_eq!("MyUnion", partial_text(&input, &stmt.name));
        let fields = &stmt.fields;
        assert_eq!(3, fields.len());
        assert_eq!("str_field", partial_text(&input, &fields[0].name));
        assert_eq!("pair_field", partial_text(&input, &fields[1].name));
        assert_eq!("int64_field", partial_text(&input, &fields[2].name));
    }

    #[test]
    fn test_parse() {
        let input = r#"
        module test.module;
        import "a.b.c";
        import "a.c.d";

        enum MyEnum;
        enum MyEnum2 { kFoo, kBar, kBaz };

        const string kMyConst = "const_value";
        const int32 kMyConst2 = -1;

        struct MyStruct {};
        struct MyStruct2 {
            uint8 my_uint8_value;
            float my_float_value = 0.1;
        };

        union MyUnion {
            string str_field;
            uint8 uint8_field;
        };
        union MyUnion2 {
            MyInterface myinterface_field;
            uint32 uint32_field;
        };

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
        assert_eq!(16, res.stmts.len());
    }
}
