use std::error::Error as StdError;
use std::io;
use std::{error, fmt};

use nom;
use serde::de;

/// The result of a serialization or deserialization operation.
pub type Result<T> = ::std::result::Result<T, Error>;

/// An error that can be produced during (de)serializing.
pub type Error = Box<ErrorKind>;

/// The kind of error that can be produced during a serialization or deserialization.
/// TODO: Handle errors produced by nom cleanly
#[derive(Debug)]
pub enum ErrorKind {
    /// Errors tied to IO issues and not the actual parsing steps.
    Io(io::Error),
    /// A custom error message from Serde.
    Custom(String),
    /// Serde has a deserialize_any method that lets the format hint to the
    /// object which route to take in deserializing.
    DeserializeAnyNotSupported,
    /// Errors generated by trying to parse invalid data with a nom combinator
    ParseError(usize, nom::error::ErrorKind),
    /// Errors tied to insufficent data in the buffer, similar to an IO error but coming from nom
    UnexpectedEof(nom::Needed)
}

impl StdError for ErrorKind {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ErrorKind::Io(ref err) => Some(err),
            ErrorKind::Custom(_) => None,
            ErrorKind::DeserializeAnyNotSupported => None,
            ErrorKind::ParseError(..) => None,
            ErrorKind::UnexpectedEof(..) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        ErrorKind::Io(err).into()
    }
}

impl From<nom::Err<(&[u8], nom::error::ErrorKind)>> for Error {
    fn from(err: nom::Err<(&[u8], nom::error::ErrorKind)>) -> Error {
        match err {
            nom::Err::Error((remaining, kind)) => ErrorKind::ParseError(remaining.len(), kind).into(),
            nom::Err::Failure((remaining, kind)) => ErrorKind::ParseError(remaining.len(), kind).into(),
            nom::Err::Incomplete(needed) => ErrorKind::UnexpectedEof(needed).into(),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ErrorKind::Io(ref ioerr) => write!(fmt, "io error: {}", ioerr),
            ErrorKind::DeserializeAnyNotSupported => write!(fmt, "FIT doesn't support serde::Deserializer::deserialize_any method"),
            ErrorKind::Custom(ref s) => s.fmt(fmt),
            ErrorKind::ParseError(rem, ref err) => write!(fmt, "parser error: '{}' bytes remaining: {}", err.description(), rem),
            ErrorKind::UnexpectedEof(nom::Needed::Size(n)) => write!(fmt, "parser error: requires {} more bytes", n),
            ErrorKind::UnexpectedEof(nom::Needed::Unknown) => write!(fmt, "parser error: requires more data"),
        }
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(desc: T) -> Error {
        ErrorKind::Custom(desc.to_string()).into()
    }
}
