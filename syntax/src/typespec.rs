use pest::Parser;

use crate::parser::{consume_token, MojomParser, Pairs, Rule};

// TODO: Support pending_receiver<T> and pending_remote<T>.
#[derive(Debug, PartialEq)]
pub enum TypeName {
    FixedArray(Box<TypeSpec>, u64 /* size */),
    Array(Box<TypeSpec>),
    Map(String, Box<TypeSpec>),
    InterfaceRequest(String, bool /* associated */),
    Handle(Option<String>),
    Associated(String),
    BasicTypeName(String),
}

fn into_handle(mut pairs: Pairs) -> TypeName {
    consume_token(Rule::t_handle, &mut pairs);
    let mut specific_handle_type = None;
    if let Some(token) = pairs.next() {
        assert_eq!(Rule::t_langlebracket, token.as_rule());
        specific_handle_type = Some(pairs.next().unwrap().as_str().to_owned());
        consume_token(Rule::t_ranglebracket, &mut pairs);
    }
    TypeName::Handle(specific_handle_type)
}

fn into_basic_name(mut pairs: Pairs) -> TypeName {
    let item = pairs.next().unwrap();
    match item.as_rule() {
        Rule::numeric_type => TypeName::BasicTypeName(item.as_str().to_owned()),
        Rule::handle_type => into_handle(item.into_inner()),
        Rule::t_associated => {
            let ident = pairs.next().unwrap().as_str().to_owned();
            TypeName::Associated(ident)
        }
        Rule::identifier => TypeName::BasicTypeName(item.as_str().to_owned()),
        _ => unreachable!(),
    }
}

fn into_array(mut pairs: Pairs) -> TypeName {
    consume_token(Rule::t_array, &mut pairs);
    consume_token(Rule::t_langlebracket, &mut pairs);
    let type_spec = into_type_spec(pairs.next().unwrap().into_inner());
    consume_token(Rule::t_ranglebracket, &mut pairs);
    TypeName::Array(Box::new(type_spec))
}

fn into_fixed_array(mut pairs: Pairs) -> TypeName {
    consume_token(Rule::t_array, &mut pairs);
    consume_token(Rule::t_langlebracket, &mut pairs);
    let type_spec = into_type_spec(pairs.next().unwrap().into_inner());
    consume_token(Rule::t_comma, &mut pairs);
    let size = pairs.next().unwrap().as_str().parse::<u64>().unwrap();
    consume_token(Rule::t_ranglebracket, &mut pairs);
    TypeName::FixedArray(Box::new(type_spec), size)
}

fn into_map(mut pairs: Pairs) -> TypeName {
    consume_token(Rule::t_map, &mut pairs);
    consume_token(Rule::t_langlebracket, &mut pairs);
    let key_type = pairs.next().unwrap().as_str().to_owned();
    consume_token(Rule::t_comma, &mut pairs);
    let value_type = into_type_spec(pairs.next().unwrap().into_inner());
    consume_token(Rule::t_ranglebracket, &mut pairs);
    TypeName::Map(key_type, Box::new(value_type))
}

fn into_interface_request(mut pairs: Pairs) -> TypeName {
    let item = pairs.next().unwrap();
    let (ident, is_associated) = match item.as_rule() {
        Rule::t_associated => {
            let ident = pairs.next().unwrap().as_str().to_owned();
            (ident, true)
        }
        Rule::identifier => (item.as_str().to_owned(), false),
        _ => unreachable!(),
    };
    consume_token(Rule::t_amp, &mut pairs);
    TypeName::InterfaceRequest(ident, is_associated)
}

fn into_type_name(mut pairs: Pairs) -> TypeName {
    let item = pairs.next().unwrap();
    match item.as_rule() {
        Rule::fixed_array => into_fixed_array(item.into_inner()),
        Rule::array => into_array(item.into_inner()),
        Rule::map => into_map(item.into_inner()),
        Rule::interface_request => into_interface_request(item.into_inner()),
        Rule::basic_type_name => into_basic_name(item.into_inner()),
        _ => unreachable!(),
    }
}

#[derive(Debug, PartialEq)]
pub struct TypeSpec {
    pub type_name: TypeName,
    pub is_nullable: bool,
}

fn into_type_spec(mut pairs: Pairs) -> TypeSpec {
    let type_name = into_type_name(pairs.next().unwrap().into_inner());

    let mut is_nullable = false;
    for item in pairs {
        match item.as_rule() {
            Rule::t_nullable => is_nullable = true,
            _ => unreachable!(),
        }
    }

    TypeSpec {
        type_name: type_name,
        is_nullable: is_nullable,
    }
}

pub fn typespec(input: &str) -> anyhow::Result<TypeSpec> {
    let mut pairs = MojomParser::parse(Rule::type_spec, input)?;
    let inner = pairs.next().unwrap().into_inner();
    Ok(into_type_spec(inner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typespec() {
        typespec("bool").unwrap();
        typespec("int8").unwrap();
        typespec("uint8").unwrap();
        typespec("int16").unwrap();
        typespec("uint16").unwrap();
        typespec("int32").unwrap();
        typespec("uint32").unwrap();
        typespec("int64").unwrap();
        typespec("uint64").unwrap();
        typespec("float").unwrap();
        typespec("double").unwrap();
        typespec("string").unwrap();

        let res = typespec("handle").unwrap();
        match res.type_name {
            TypeName::Handle(handle_type) => {
                assert!(handle_type.is_none());
            }
            _ => panic!("Expected handle"),
        };

        let res = typespec("handle<message_pipe>").unwrap();
        match res.type_name {
            TypeName::Handle(handle_type) => {
                assert_eq!(handle_type.unwrap(), "message_pipe");
            }
            _ => panic!("Expected handle"),
        };

        let res = typespec("array<uint8>").unwrap();
        match res.type_name {
            TypeName::Array(inner_type) => {
                assert_eq!(
                    TypeName::BasicTypeName("uint8".to_owned()),
                    inner_type.type_name
                );
            }
            _ => panic!("Expected array"),
        };

        let res = typespec("array<string, 16>").unwrap();
        match res.type_name {
            TypeName::FixedArray(inner_type, size) => {
                assert_eq!(
                    TypeName::BasicTypeName("string".to_owned()),
                    inner_type.type_name
                );
                assert_eq!(16, size);
            }
            _ => panic!("Expected fixed array"),
        };

        let res = typespec("map<int32, MyInterface>").unwrap();
        match res.type_name {
            TypeName::Map(key_type, value_type) => {
                assert_eq!("int32", key_type);
                assert_eq!(
                    TypeName::BasicTypeName("MyInterface".to_owned()),
                    value_type.type_name
                );
            }
            _ => panic!("Expected map"),
        };

        typespec("MyInterface").unwrap();

        let res = typespec("MyInterface&").unwrap();
        match res.type_name {
            TypeName::InterfaceRequest(ident, is_associated) => {
                assert_eq!("MyInterface", ident);
                assert!(!is_associated);
            }
            _ => panic!("Expected interface request"),
        };

        let res = typespec("associated MyInterface&").unwrap();
        match res.type_name {
            TypeName::InterfaceRequest(ident, is_associated) => {
                assert_eq!("MyInterface", ident);
                assert!(is_associated);
            }
            _ => panic!("Expected interface request"),
        };

        let res = typespec("associated MyInterface").unwrap();
        match res.type_name {
            TypeName::Associated(ident) => {
                assert_eq!("MyInterface", ident);
            }
            _ => panic!("Expected associated type"),
        };

        let input = "array<int32>?";
        let res = typespec(&input).unwrap();
        assert!(res.is_nullable);
    }
}
