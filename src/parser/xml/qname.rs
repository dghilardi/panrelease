//! # xrust::qname
//!
//! Support for Qualified Names.

use core::hash::{Hash, Hasher};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub struct QualifiedName {
    nsuri: Option<String>,
    prefix: Option<String>,
    localname: String,
}

// TODO: we may need methods that return a string slice, rather than a copy of the string
impl QualifiedName {
    pub fn new(nsuri: Option<String>, prefix: Option<String>, localname: String) -> QualifiedName {
        QualifiedName {
            nsuri,
            prefix,
            localname,
        }
    }
    pub fn get_localname(&self) -> String {
        self.localname.clone()
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        self.prefix.as_ref().map_or((), |p| {
            result.push_str(p.as_str());
            result.push(':');
        });
        result.push_str(self.localname.as_str());
        write!(f, "{}", result)
    }
}

impl PartialEq for QualifiedName {
    // Only the namespace URI and local name have to match
    fn eq(&self, other: &QualifiedName) -> bool {
        self.nsuri.as_ref().map_or_else(
            || {
                other
                    .nsuri
                    .as_ref()
                    .map_or_else(|| self.localname.eq(other.localname.as_str()), |_| false)
            },
            |ns| {
                other.nsuri.as_ref().map_or_else(
                    || false,
                    |ons| ns.eq(ons.as_str()) && self.localname.eq(other.localname.as_str()),
                )
            },
        )
    }
}
impl Eq for QualifiedName {}

impl Hash for QualifiedName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(ref ns) = self.nsuri {
            ns.hash(state);
        }
        self.localname.hash(state);
    }
}