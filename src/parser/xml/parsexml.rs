//! # xdm::parsexml
//!
//! A parser for XML, as a nom parser combinator.
//! XML 1.1, see <https://www.w3.org/TR/xml11/>
//!
//! This is a very simple, minimalist parser of XML. It excludes:
//! DTDs (and therefore entities)

extern crate nom;

use std::collections::HashSet;
use std::convert::TryFrom;
use std::str::FromStr;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while_m_n},
    character::complete::{char, digit1, hex_digit1, multispace0, multispace1, none_of},
    combinator::{map, map_opt, opt, recognize, value, verify},
    IResult,
    multi::{many0, many1},
    sequence::delimited,
    sequence::tuple,
};
use nom::bytes::complete::{escaped, take_while1};
use nom::character::complete::{one_of, satisfy};
use nom::combinator::cut;
use nom::error::{context, ParseError};
use nom::sequence::{preceded, terminated};

use crate::parser::xml::value::StringRepr;

use super::parsecommon::*;
use super::qname::*;
use super::value::Value;
use super::xdmerror::*;

// nom doesn't pass additional parameters, only the input,
// so this is a two-pass process.
// First, use nom to tokenize and parse the input.
// Second, use the internal structure returned by the parser
// to build the document structure.

// This structure allows multiple root elements.
// An XML document will only be well-formed if there is exactly one element.
// However, external general entities may have more than one element.
pub struct XMLDocument<'a> {
    pub prologue: Vec<XMLNode<'a>>,
    pub content: Vec<XMLNode<'a>>,
    pub epilogue: Vec<XMLNode<'a>>,
    pub xmldecl: Option<XMLdecl>,
}

impl <'a> TryFrom<&'a str> for XMLDocument<'a> {
    type Error = Error;
    fn try_from(e: &'a str) -> Result<Self, Self::Error> {
        match document(e.trim()) {
            Ok((rest, value)) => {
                if rest.is_empty() {
                    Result::Ok(value)
                } else {
                    Result::Err(Error {
                        kind: ErrorKind::Unknown,
                        message: format!(
                            "extra characters after expression: \"{}\"",
                            rest
                        ),
                    })
                }
            }
            Err(nom::Err::Error(c)) => Result::Err(Error {
                kind: ErrorKind::Unknown,
                message: format!("parser error: {:?}", c),
            }),
            Err(nom::Err::Incomplete(_)) => Result::Err(Error {
                kind: ErrorKind::Unknown,
                message: String::from("incomplete input"),
            }),
            Err(nom::Err::Failure(f)) => Result::Err(Error {
                kind: ErrorKind::Unknown,
                message: format!("unrecoverable parser error - {f}"),
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum XMLNode<'a> {
    Element(QualifiedName, Vec<XMLNode<'a>>, Vec<XMLNode<'a>>), // Element name, attributes, content
    Attribute(QualifiedName, Value<'a>),
    Text(Value<'a>),
    PI(String, Value<'a>),
    Comment(Value<'a>),           // Comment value is a string
    Dtd(DtdDecl),             // These only occur in the prologue
    Reference(QualifiedName), // General entity reference. These need to be resolved before presentation to the application
}

#[derive(PartialEq, Eq)]
pub struct XMLdecl {
    version: String,
    encoding: Option<String>,
    standalone: Option<String>,
}

/// DTD declarations.
/// Only general entities are supported, so far.
/// TODO: element, attribute declarations
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DtdDecl {
    GeneralEntity(QualifiedName, String),
}

// document ::= ( prolog element misc*)
fn document(input: &str) -> IResult<&str, XMLDocument> {
    map(tuple((opt(prolog), multispace0, element, opt(misc))), |(p, _, e, m)| {
        let pr = p.unwrap_or((None, vec![]));

        XMLDocument {
            content: vec![e],
            epilogue: m.unwrap_or_default(),
            xmldecl: pr.0,
            prologue: pr.1,
        }
    })(input)
}

// prolog ::= XMLDecl misc* (doctypedecl Misc*)?
fn prolog(input: &str) -> IResult<&str, (Option<XMLdecl>, Vec<XMLNode>)> {
    map(tuple((opt(xmldecl), opt(doctypedecl))), |(x, dtd)| {
        (x, dtd.map_or(vec![], |d| d))
    })(input)
}

fn xmldecl(input: &str) -> IResult<&str, XMLdecl> {
    map(
        tuple((
            tag("<?xml"),
            multispace0,
            map(
                tuple((
                    tag("version"),
                    multispace0,
                    tag("="),
                    multispace0,
                    delimited_string,
                )),
                |(_, _, _, _, v)| v,
            ),
            multispace0,
            opt(map(
                tuple((
                    tag("encoding"),
                    multispace0,
                    tag("="),
                    multispace0,
                    delimited_string,
                )),
                |(_, _, _, _, e)| e,
            )),
            multispace0,
            opt(map(
                tuple((
                    tag("standalone"),
                    multispace0,
                    tag("="),
                    multispace0,
                    delimited_string,
                )),
                |(_, _, _, _, s)| s,
            )),
            multispace0,
            tag("?>"),
        )),
        |(_, _, ver, _, enc, _, sta, _, _)| XMLdecl {
            version: String::from(ver),
            encoding: enc.map(String::from),
            standalone: sta.map(String::from),
        },
    )(input)
}

fn doctypedecl(input: &str) -> IResult<&str, Vec<XMLNode>> {
    map(
        tuple((
            tag("<!DOCTYPE"),
            multispace1,
            qualname,
            map(opt(map(tuple((multispace1, externalid)), |e| e)), |e| e),
            multispace0,
            opt(map(
                tuple((
                    tag("["),
                    multispace0,
                    intsubset,
                    multispace0,
                    tag("]"),
                    multispace0,
                )),
                |(_, _, i, _, _, _)| i,
            )),
            tag(">"),
        )),
        |(_, _, _n, _extid, _, intss, _)| {
            // TODO: the name must match the document element
            intss.map_or(vec![], |i| i)
        },
    )(input)
}

// TODO: parameter entities
// intSubset ::= (markupdecl | DeclSep)*
// markupdecl ::= elementdecl | AttlistDecl | EntityDecl | NotationDecl | PI | Comment
fn intsubset(input: &str) -> IResult<&str, Vec<XMLNode>> {
    many0(alt((entitydecl, processing_instruction, comment)))(input)
}

// EntityDecl ::= GEDecl | PEDecl
// TODO: support parameter entities
fn entitydecl(input: &str) -> IResult<&str, XMLNode> {
    // TODO: handle quotes properly
    map(
        tuple((
            tag("<!ENTITY"),
            multispace1,
            qualname,
            multispace1,
            entityvalue,
            multispace0,
            tag(">"),
        )),
        |(_, _, n, _, v, _, _)| XMLNode::Dtd(DtdDecl::GeneralEntity(n, v)),
    )(input)
}

fn entityvalue(input: &str) -> IResult<&str, String> {
    alt((entityvalue_single, entityvalue_double))(input)
}
// TODO: parameter entity references
fn entityvalue_single(input: &str) -> IResult<&str, String> {
    map(
        delimited(
            char('\''),
            recognize(many0(alt((
                map(recognize(reference), String::from),
                map(many1(none_of("'&")), |v| v.iter().collect::<String>()),
            )))),
            char('\''),
        ),
        String::from,
    )(input)
}
fn entityvalue_double(input: &str) -> IResult<&str, String> {
    map(
        delimited(
            char('"'),
            recognize(many0(alt((
                map(recognize(reference), String::from),
                map(many1(none_of("\"&")), |v| v.iter().collect::<String>()),
            )))),
            char('"'),
        ),
        String::from,
    )(input)
}

fn externalid(input: &str) -> IResult<&str, Vec<XMLNode>> {
    map(tag("not yet implemented"), |_| {
        vec![XMLNode::Text(Value::String(
            "external ID not yet implemented"
        ))]
    })(input)
}

// Element ::= EmptyElemTag | STag content ETag
fn element(input: &str) -> IResult<&str, XMLNode> {
    map(alt((emptyelem, taggedelem)), |e| {
        // TODO: Check for namespace declarations, and resolve URIs in the node tree under 'e'
        e
    })(input)
}

// STag ::= '<' Name (Attribute)* '>'
// ETag ::= '</' Name '>'
// NB. Names must match
fn taggedelem(input: &str) -> IResult<&str, XMLNode> {
    map(
        tuple((
            tag("<"),
            qualname,
            attributes, //many0(attribute),
            multispace0,
            tag(">"),
            multispace0,
            content,
            tag("</"),
            qualname,
            multispace0,
            tag(">"),
        )),
        |(_, n, a, _, _, _, c, _, _e, _, _)| {
            // TODO: check that the start tag name and end tag name match (n == e)
            XMLNode::Element(n, a, c)
        },
    )(input)
}

// EmptyElemTag ::= '<' Name (Attribute)* '/>'
fn emptyelem(input: &str) -> IResult<&str, XMLNode> {
    map(
        tuple((
            tag("<"),
            qualname,
            attributes, //many0(attribute),
            multispace0,
            tag("/>"),
        )),
        |(_, n, a, _, _)| XMLNode::Element(n, a, vec![]),
    )(input)
}

fn attributes(input: &str) -> IResult<&str, Vec<XMLNode>> {
    //this is just a wrapper around the attribute function, that checks for duplicates.
    verify(many0(attribute), |v: &[XMLNode]| {
        let attrs = &v.iter().collect::<Vec<_>>();
        let uniqueattrs: HashSet<_> = attrs
            .iter()
            .map(|xmlnode| match xmlnode {
                XMLNode::Attribute(q, _) => q.to_string(),
                _ => "".to_string(),
            })
            .collect();

        v.len() == uniqueattrs.len()
    })(input)
}

// Attribute ::= Name '=' AttValue
fn attribute(input: &str) -> IResult<&str, XMLNode> {
    map(
        tuple((
            multispace1,
            qualname,
            multispace0,
            tag("="),
            multispace0,
            delimited_string,
        )),
        |(_, n, _, _, _, s)| XMLNode::Attribute(n, Value::String(s)),
    )(input)
}
fn delimited_string(input: &str) -> IResult<&str, &str> {
    alt((string_single, string_double))(input)
}
fn string_single(input: &str) -> IResult<&str, &str> {
    context(
        "string",
        preceded(char('\''), cut(terminated(parse_str, char('\'')))),
    )(input)
}
fn string_double(input: &str) -> IResult<&str, &str> {
    context(
        "string",
        preceded(char('\"'), cut(terminated(parse_str, char('\"')))),
    )(input)
}

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    escaped(satisfy(|c| c.is_alphanumeric() || c.is_whitespace() || ['-', '.', ':', '/', '$', '{', '}', '*'].contains(&c)), '\\', one_of("\"n\\"))(i)
}

// content ::= CharData? ((element | Reference | CDSect | PI | Comment) CharData?)*
fn content(input: &str) -> IResult<&str, Vec<XMLNode>> {
    map(
        tuple((
            opt(chardata),
            many0(tuple((
                alt((
                    element,
                    reference,
                    // TODO: CData Section
                    processing_instruction,
                    comment,
                )),
                opt(chardata),
            ))),
        )),
        |(m_c, v)| {
            let mut new: Vec<XMLNode> = Vec::new();
            if let Some(c) = m_c {
                c.into_iter()
                    .map(Value::from)
                    .map(XMLNode::Text)
                    .for_each(|n| new.push(n));
            }
            if !v.is_empty() {
                for (w, m_d) in v {
                    new.push(w);
                    if let Some(d) = m_d {
                        d.into_iter()
                            .map(Value::from)
                            .map(XMLNode::Text)
                            .for_each(|n| new.push(n));
                    }
                }
            }
            new
        },
    )(input)
}

// Reference ::= EntityRef | CharRef
fn reference(input: &str) -> IResult<&str, XMLNode> {
    alt((entityref, charref))(input)
}
fn entityref(input: &str) -> IResult<&str, XMLNode> {
    map(tuple((char('&'), qualname, char(';'))), |(_, n, _)| {
        XMLNode::Reference(n)
    })(input)
}
fn charref(input: &str) -> IResult<&str, XMLNode> {
    alt((charref_octal, charref_hex))(input)
}
fn charref_octal(input: &str) -> IResult<&str, XMLNode> {
    map(
        tuple((char('&'), char('#'), digit1, char(';'))),
        |(_, _, n, _)| {
            let u = match u32::from_str_radix(n, 8) {
                Ok(c) => c,
                Err(_) => 0, // TODO: pass back error to nom
            };
            match std::char::from_u32(u) {
                Some(c) => XMLNode::Text(Value::StringOwned(c.to_string())),
                None => {
                    //make_error(input, NomErrorKind::OctDigit)
                    XMLNode::Text(Value::String(""))
                }
            }
        },
    )(input)
}
fn charref_hex(input: &str) -> IResult<&str, XMLNode> {
    map(
        tuple((char('&'), char('#'), char('x'), hex_digit1, char(';'))),
        |(_, _, _, n, _)| {
            let u = match u32::from_str_radix(n, 16) {
                Ok(c) => c,
                Err(_) => 0, // TODO: pass back error to nom
            };
            match std::char::from_u32(u) {
                Some(c) => XMLNode::Text(Value::StringOwned(c.to_string())),
                None => {
                    //make_error(input, NomErrorKind::OctDigit)
                    XMLNode::Text(Value::String(""))
                }
            }
        },
    )(input)
}

// PI ::= '<?' PITarget (char* - '?>') '?>'
fn processing_instruction(input: &str) -> IResult<&str, XMLNode> {
    map(
        delimited(
            tag("<?"),
            tuple((multispace0, name, multispace0, take_until("?>"))),
            tag("?>"),
        ),
        |(_, n, _, v)| XMLNode::PI(String::from(n), Value::String(v)),
    )(input)
}

// Comment ::= '<!--' (char* - '--') '-->'
fn comment(input: &str) -> IResult<&str, XMLNode> {
    map(
        delimited(tag("<!--"), take_until("--"), tag("-->")),
        |v: &str| XMLNode::Comment(Value::String(v)),
    )(input)
}

// Misc ::= Comment | PI | S
fn misc(input: &str) -> IResult<&str, Vec<XMLNode>> {
    map(tag("not yet implemented"), |_| {
        //vec![Node::new(NodeType::Comment).set_value("not yet implemented".to_string())]
        vec![]
    })(input)
}

// CharData ::= [^<&]* - (']]>')
fn chardata(input: &str) -> IResult<&str, Vec<StringRepr>> {
    many1(alt((chardata_cdata, chardata_escapes, chardata_literal)))(input)
}

fn chardata_cdata(input: &str) -> IResult<&str, StringRepr> {
    map(
        delimited(tag("<![CDATA["), take_until("]]>"), tag("]]>")),
        StringRepr::Str,
    )(input)
}

fn chardata_escapes(input: &str) -> IResult<&str, StringRepr> {
    map(alt((
        chardata_unicode_codepoint,
        value(">".to_string(), tag("&gt;")),
        value("<".to_string(), tag("&lt;")),
        value("&".to_string(), tag("&amp;")),
        value("\"".to_string(), tag("&quot;")),
        value("\'".to_string(), tag("&apos;")),
    )), StringRepr::String)(input)
}

fn chardata_unicode_codepoint(input: &str) -> IResult<&str, String> {
    let parse_hex = map(
        take_while_m_n(1, 6, |c: char| c.is_ascii_hexdigit()),
        |hex| u32::from_str_radix(hex, 16),
    );

    let parse_decimal = map(take_while_m_n(1, 6, |c: char| c.is_ascii_digit()), |dec| {
        u32::from_str(dec)
    });

    map_opt(
        alt((
            delimited(tag("&#x"), parse_hex, tag(";")),
            delimited(tag("&#"), parse_decimal, tag(";")),
        )),
        |value| Option::from(std::char::from_u32(value.unwrap()).unwrap().to_string()),
    )(input)
}

fn chardata_literal(input: &str) -> IResult<&str, StringRepr> {
    map(
        verify(take_while1(|c| c != '<' && c != '&'), |v: &str| {
            // chardata cannot contain ]]>
            let cd_end = "]]>";
            let mut w = v;
            while !w.is_empty() {
                if w.starts_with(cd_end) {
                    return false;
                }
                if !is_char(&w.chars().next().unwrap()) {
                    return false;
                }
                w = &w[1..];
            }
            true
        }),
        StringRepr::Str,
    )(input)
}

// QualifiedName
fn qualname(input: &str) -> IResult<&str, QualifiedName> {
    alt((prefixed_name, unprefixed_name))(input)
}
fn unprefixed_name(input: &str) -> IResult<&str, QualifiedName> {
    map(ncname, |localpart| {
        QualifiedName::new(None, None, String::from(localpart))
    })(input)
}
fn prefixed_name(input: &str) -> IResult<&str, QualifiedName> {
    map(
        tuple((ncname, tag(":"), ncname)),
        |(prefix, _, localpart)| {
            QualifiedName::new(None, Some(String::from(prefix)), String::from(localpart))
        },
    )(input)
}