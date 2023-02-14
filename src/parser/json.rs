use std::collections::HashMap;

use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{alphanumeric1 as alphanumeric, char, one_of},
    combinator::{cut, map, opt, value},
    error::{context, ContextError, ErrorKind, ParseError},
    IResult,
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated},
};
use nom::character::complete::satisfy;

use crate::parser::FormatCodec;
use crate::utils::get_range;

pub struct JsonString {
    inner: String,
}

impl JsonString {
    pub fn new(input: &str) -> Self {
        Self {
            inner: String::from(input),
        }
    }
}

impl FormatCodec for JsonString {
    fn extract(&self, path: &str) -> anyhow::Result<Option<&str>> {
        let (_, parsed) = root::<(&str, ErrorKind)>(&self.inner)
            .map_err(|e| anyhow!("Error during parsing {e}"))?;
        let path_parts = path.split('.');

        let mut next = &parsed;
        for key in path_parts {
            let JsonValue::Object(obj) = next else {
                anyhow::bail!("Parsed value is not an object");
            };
            let Some(value) = obj.get(key) else {
                anyhow::bail!("Could not find key {key}");
            };
            next = value;
        }
        let JsonValue::Str(value) = next else {
            anyhow::bail!("Could not find {path}")
        };
        Ok(Some(*value))
    }

    fn replace(&mut self, path: &str, value: &str) -> anyhow::Result<()> {
        let Some(current) = self.extract(path)? else {
            anyhow::bail!("Could not find {path} in given json")
        };
        let (start, end) = get_range(&self.inner, current);
        self.inner.replace_range(start..end, value);
        Ok(())
    }
}

impl ToString for JsonString {
    fn to_string(&self) -> String {
        self.inner.clone()
    }
}

#[derive(Debug, PartialEq)]
pub enum JsonValue<'a> {
    Null,
    Str(&'a str),
    Boolean(bool),
    Num(f64),
    Array(Vec<JsonValue<'a>>),
    Object(HashMap<String, JsonValue<'a>>),
}

fn space<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";
    take_while(move |c| chars.contains(c))(i)
}

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    escaped(satisfy(|c| !c.is_control() && !['\\', '"'].contains(&c)), '\\', one_of("\"n\\"))(i)
}

fn boolean<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, bool, E> {
    let parse_true = value(true, tag("true"));

    let parse_false = value(false, tag("false"));

    alt((parse_true, parse_false))(input)
}

fn null<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    value((), tag("null"))(input)
}

fn string<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, &'a str, E> {
    context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str, char('\"')))),
    )(i)
}

fn array<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<JsonValue>, E> {
    context(
        "array",
        preceded(
            char('['),
            cut(terminated(
                separated_list0(preceded(space, char(',')), json_value),
                preceded(space, char(']')),
            )),
        ),
    )(i)
}

fn key_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, (&'a str, JsonValue), E> {
    separated_pair(
        preceded(space, string),
        cut(preceded(space, char(':'))),
        json_value,
    )(i)
}

fn hash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, HashMap<String, JsonValue>, E> {
    context(
        "map",
        preceded(
            char('{'),
            cut(terminated(
                map(
                    separated_list0(preceded(space, char(',')), key_value),
                    |tuple_vec| {
                        tuple_vec
                            .into_iter()
                            .map(|(k, v)| (String::from(k), v))
                            .collect()
                    },
                ),
                preceded(space, char('}')),
            )),
        ),
    )(i)
}

fn json_value<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, JsonValue, E> {
    preceded(
        space,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(string, JsonValue::Str),
            map(double, JsonValue::Num),
            map(boolean, JsonValue::Boolean),
            map(null, |_| JsonValue::Null),
        )),
    )(i)
}

fn root<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, JsonValue, E> {
    delimited(
        space,
        alt((
            map(hash, JsonValue::Object),
            map(array, JsonValue::Array),
            map(null, |_| JsonValue::Null),
        )),
        opt(space),
    )(i)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_json_extraction() {
        let input = JsonString::new(r#"{"hola":"world", "hey":{"hello":"world"}, "goodbye":"world."}"#);
        let extracted = input.extract("hey.hello")
            .expect("Error extracting value")
            .expect("Value not found");

        let (start, end) = get_range(&input.inner, extracted);

        let mut replaced = input.inner.clone();
        replaced.replace_range(start..end, "asd");
        println!("{}", replaced);

        assert_eq!("world", extracted);
    }
}