//! # xrust::value
//!
//! An atomic value as an item in a sequence.

use core::fmt;
use std::convert::TryFrom;

use super::xdmerror::{Error, ErrorKind};

/// A concrete type that implements atomic values.
/// These are the 19 predefined types in XSD Schema Part 2, plus five additional types.
#[derive(Clone, Debug)]
pub enum Value<'a> {
    String(&'a str),
    StringOwned(String),
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
