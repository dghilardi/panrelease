//! # xrust::value
//!
//! An atomic value as an item in a sequence.

use super::xdmerror::{Error, ErrorKind};
use chrono::{Date, DateTime, Local};
use core::fmt;
use std::cmp::Ordering;
use std::convert::TryFrom;

/// Comparison operators for values
#[derive(Copy, Clone)]
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Is,
    Before,
    After,
}

impl Operator {
    pub fn to_string(&self) -> &str {
        match self {
            Operator::Equal => "=",
            Operator::NotEqual => "!=",
            Operator::LessThan => "<",
            Operator::LessThanEqual => "<=",
            Operator::GreaterThan => ">",
            Operator::GreaterThanEqual => ">=",
            Operator::Is => "is",
            Operator::Before => "<<",
            Operator::After => ">>",
        }
    }
}

/// A concrete type that implements atomic values.
/// These are the 19 predefined types in XSD Schema Part 2, plus five additional types.
#[derive(Clone, Debug)]
pub enum Value<'a> {
    /// node or simple type
    AnyType,
    /// a not-yet-validated anyType
    Untyped,
    /// base type of all simple types. i.e. not a node
    AnySimpleType,
    /// a list of IDREF
    IDREFS,
    /// a list of NMTOKEN
    NMTOKENS,
    /// a list of ENTITY
    ENTITIES,
    /// Any numeric type
    Numeric,
    /// all atomic values (no lists or unions)
    AnyAtomicType,
    /// untyped atomic value
    UntypedAtomic,
    Duration,
    Time(DateTime<Local>), // Ignore the date part. Perhaps use Instant instead?
    Float(f32),
    Double(f64),
    Integer(i64),
    NonPositiveInteger(NonPositiveInteger),
    NegativeInteger(NegativeInteger),
    Long(i64),
    Int(i32),
    Short(i16),
    Byte(i8),
    NonNegativeInteger(NonNegativeInteger),
    UnsignedLong(u64),
    UnsignedInt(u32),
    UnsignedShort(u16),
    UnsignedByte(u8),
    PositiveInteger(PositiveInteger),
    DateTime(DateTime<Local>),
    DateTimeStamp,
    Date(Date<Local>),
    String(&'a str),
    StringOwned(String),
    NormalizedString(NormalizedString),
    /// Like normalizedString, but without leading, trailing and consecutive whitespace
    Token,
    /// language identifiers [a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*
    Language,
    /// NameChar+
    NMTOKEN,
    /// NameStartChar NameChar+
    Name,
    /// (Letter | '_') NCNameChar+ (i.e. a Name without the colon)
    NCName,
    /// Same format as NCName
    ID,
    /// Same format as NCName
    IDREF,
    /// Same format as NCName
    ENTITY,
    Boolean(bool),
}

pub enum StringRepr<'a> {
    String(String),
    Str(&'a str),
}

impl <'a> From<StringRepr<'a>> for Value<'a> {
    fn from(s: StringRepr<'a>) -> Self {
        match s {
            StringRepr::String(s) => Value::StringOwned(s),
            StringRepr::Str(s) => Value::String(s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NonPositiveInteger(i64);
impl TryFrom<i64> for NonPositiveInteger {
    type Error = Error;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v > 0 {
            Err(Error::new(
                ErrorKind::TypeError,
                String::from("NonPositiveInteger must be less than zero"),
            ))
        } else {
            Ok(NonPositiveInteger(v))
        }
    }
}
impl fmt::Display for NonPositiveInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct PositiveInteger(i64);
impl TryFrom<i64> for PositiveInteger {
    type Error = Error;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v <= 0 {
            Err(Error::new(
                ErrorKind::TypeError,
                String::from("PositiveInteger must be greater than zero"),
            ))
        } else {
            Ok(PositiveInteger(v))
        }
    }
}
impl fmt::Display for PositiveInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct NonNegativeInteger(i64);
impl TryFrom<i64> for NonNegativeInteger {
    type Error = Error;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v < 0 {
            Err(Error::new(
                ErrorKind::TypeError,
                String::from("NonNegativeInteger must be zero or greater"),
            ))
        } else {
            Ok(NonNegativeInteger(v))
        }
    }
}
impl fmt::Display for NonNegativeInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct NegativeInteger(i64);
impl TryFrom<i64> for NegativeInteger {
    type Error = Error;
    fn try_from(v: i64) -> Result<Self, Self::Error> {
        if v >= 0 {
            Err(Error::new(
                ErrorKind::TypeError,
                String::from("NegativeInteger must be less than zero"),
            ))
        } else {
            Ok(NegativeInteger(v))
        }
    }
}
impl fmt::Display for NegativeInteger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct NormalizedString(String);
impl TryFrom<&str> for NormalizedString {
    type Error = Error;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let n: &[_] = &['\n', '\r', '\t'];
        if v.find(n).is_none() {
            Ok(NormalizedString(v.to_string()))
        } else {
            Err(Error::new(
                ErrorKind::TypeError,
                String::from("value is not a normalized string"),
            ))
        }
    }
}
impl fmt::Display for NormalizedString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0.to_string())
    }
}
