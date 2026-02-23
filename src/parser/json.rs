use std::collections::HashMap;

use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while},
    character::complete::{char, one_of},
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

fn json_type_name(v: &JsonValue<'_>) -> &'static str {
    match v {
        JsonValue::Null => "null",
        JsonValue::Str(_) => "string",
        JsonValue::Boolean(_) => "boolean",
        JsonValue::Num(_) => "number",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

impl FormatCodec for JsonString {
    fn extract(&self, path: &str) -> anyhow::Result<Option<&str>> {
        let (_, parsed) = root::<(&str, ErrorKind)>(&self.inner)
            .map_err(|e| anyhow!("JSON parse error: {e}"))?;
        let path_parts: Vec<&str> = path.split('.').collect();

        let mut next = &parsed;
        let mut traversed = String::new();
        for key in &path_parts {
            if !traversed.is_empty() {
                traversed.push('.');
            }
            traversed.push_str(key);

            let JsonValue::Object(obj) = next else {
                anyhow::bail!(
                    "Expected an object at '{}' while traversing path '{}', but found {}",
                    traversed, path, json_type_name(next)
                );
            };
            let Some(value) = obj.get(*key) else {
                let available: Vec<&str> = obj.keys().map(String::as_str).collect();
                anyhow::bail!(
                    "Key '{}' not found at '{}' while traversing path '{}'. Available keys: [{}]",
                    key, traversed, path, available.join(", ")
                );
            };
            next = value;
        }
        let JsonValue::Str(value) = next else {
            anyhow::bail!(
                "Expected a string at path '{}' but found {}",
                path, json_type_name(next)
            )
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
    escaped(
        satisfy(|c| !c.is_control() && !['\\', '"'].contains(&c)),
        '\\',
        one_of("\"\\/bfnrtu"),
    )(i)
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
    let (rest, _) = context("string", char('\"'))(i)?;
    if rest.starts_with('"') {
        // empty string ""
        Ok((&rest[1..], &rest[..0]))
    } else {
        let (rest2, content) = cut(parse_str)(rest)?;
        let (rest3, _) = cut(char('\"'))(rest2)?;
        Ok((rest3, content))
    }
}

fn array<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, Vec<JsonValue<'a>>, E> {
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
) -> IResult<&'a str, (&'a str, JsonValue<'a>), E> {
    separated_pair(
        preceded(space, string),
        cut(preceded(space, char(':'))),
        json_value,
    )(i)
}

fn hash<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
    i: &'a str,
) -> IResult<&'a str, HashMap<String, JsonValue<'a>>, E> {
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
) -> IResult<&'a str, JsonValue<'a>, E> {
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
) -> IResult<&'a str, JsonValue<'a>, E> {
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

    #[test]
    fn extract_missing_key_mentions_available_keys() {
        let input = JsonString::new(r#"{"version":"1.0.0","name":"myapp"}"#);
        let err = input.extract("missing_key").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("version") || msg.contains("name"),
            "error should list available keys: {msg}"
        );
        assert!(msg.contains("missing_key"), "error should mention the missing key: {msg}");
    }

    #[test]
    fn extract_non_object_traversal_mentions_actual_type() {
        let input = JsonString::new(r#"{"version":"1.0.0"}"#);
        // "version" is a string, not an object, so traversing "version.sub" should fail clearly
        let err = input.extract("version.sub").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("string"), "error should mention actual type: {msg}");
        assert!(msg.contains("version"), "error should mention the path: {msg}");
    }

    #[test]
    fn extract_non_string_leaf_mentions_actual_type() {
        let input = JsonString::new(r#"{"count":42}"#);
        let err = input.extract("count").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("number"), "error should mention actual type: {msg}");
        assert!(msg.contains("count"), "error should mention the path: {msg}");
    }

    #[test]
    fn parse_empty_string_values() {
        let input = JsonString::new(r#"{"name":"","version":"1.0.0"}"#);
        let ver = input.extract("version").unwrap().unwrap();
        assert_eq!(ver, "1.0.0");
    }

    #[test]
    fn parse_json_with_all_escape_sequences() {
        let input = JsonString::new(r#"{"path":"a\/b","tab":"a\tb","version":"2.0.0"}"#);
        let ver = input.extract("version").unwrap().unwrap();
        assert_eq!(ver, "2.0.0");
    }

    #[test]
    fn parse_complex_json_with_arrays_nulls_and_empty_strings() {
        let input = JsonString::new(r#"{
  "version": "0.1.0",
  "bundle": {
    "copyright": "",
    "icon": ["a.png", "b.png"],
    "macOS": {
      "entitlements": null,
      "frameworks": []
    },
    "windows": {
      "timestampUrl": ""
    }
  }
}"#);
        let ver = input.extract("version").unwrap().unwrap();
        assert_eq!(ver, "0.1.0");
    }
}