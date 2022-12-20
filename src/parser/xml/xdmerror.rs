//! # xrust::error
//!
//! XDM, XPath, XQuery and XSLT errors.

use core::fmt;

/// Errors defined in XPath
#[derive(Copy, Clone, Debug)]
pub enum ErrorKind {
    /// XPST0003
    TypeError,
    Unknown,
}

/// An error returned by an XPath, XQuery or XSLT function/method
#[derive(Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl std::error::Error for Error {}

impl Error {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Error { kind, message }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}
