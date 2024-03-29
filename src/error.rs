use std::{
    error,
    fmt::{self, Debug, Display},
    io, result,
};

/// Local Result type def
pub type Result<T> = result::Result<T, Error>;

pub struct Error {
    inner: Box<Err>,
}

impl Error {
    pub fn categorize(&self) -> Category {
        match self.inner.err {
            ErrorKind::Io(_) => Category::Io,
            ErrorKind::Message(_)
            | ErrorKind::InvalidChar
            | ErrorKind::ScalarInvalid
            | ErrorKind::SoloCarriageReturn
            | ErrorKind::InvalidEOL
            | ErrorKind::InvalidEOF => Category::Data,
            ErrorKind::IllegalTransition
            | ErrorKind::StateViolation
            | ErrorKind::EOFMapping
            | ErrorKind::RepeatFailure => Category::State,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.inner.err
    }

    pub fn context(&self) -> Option<&Context> {
        self.inner.cxt.as_ref()
    }

    /// Helper for checking if this is a repeat error
    pub(super) fn is_repeat(&self) -> bool {
        match self.inner.err {
            ErrorKind::RepeatFailure => true,
            _ => false,
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            inner: Box::new(Err::new(kind, None)),
        }
    }
}

impl<T: Into<Err>> From<T> for Error {
    fn from(err: T) -> Self {
        Self {
            inner: Box::new(err.into()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.err)?;

        if let Some(cxt) = self.inner.cxt.as_ref() {
            write!(f, " {}", cxt)?;
        }

        Ok(())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.err)?;

        if let Some(cxt) = self.inner.cxt.as_ref() {
            write!(f, " {}", cxt)?;
        }

        Ok(())
    }
}

impl error::Error for Error {}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        if let ErrorKind::Io(err) = e.inner.err {
            err
        } else {
            match e.categorize() {
                Category::Io => unreachable!(),
                Category::Data => io::Error::new(io::ErrorKind::InvalidData, e),
                Category::State => io::Error::new(io::ErrorKind::InvalidInput, e),
            }
        }
    }
}

#[derive(Debug)]
pub struct Err {
    err: ErrorKind,
    cxt: Option<Context>,
}

impl Err {
    pub(super) fn new(err: ErrorKind, cxt: Option<Context>) -> Self {
        Self { err, cxt }
    }

    pub(super) fn with_context<T: Into<Context>>(self, cxt: T) -> Self {
        Self::new(self.err, Some(cxt.into()))
    }
}

impl From<io::Error> for Err {
    fn from(e: io::Error) -> Self {
        Self::new(ErrorKind::Io(e), None)
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    Message(Box<str>),

    Io(io::Error),

    // Default error returned when parser has already errored
    RepeatFailure,

    IllegalTransition,

    StateViolation,

    InvalidChar,

    ScalarInvalid,

    EOFMapping,

    InvalidEOL,

    InvalidEOF,

    SoloCarriageReturn,
}

impl ErrorKind {
    pub(super) fn with_context<T: Into<Context>>(self, cxt: T) -> Err {
        Err::new(self, Some(cxt.into()))
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{}", msg),
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::RepeatFailure => write!(f, "Attempted to drive an already failed parser"),
            Self::IllegalTransition => write!(
                f,
                "Parser attempted an illegal state transition... this is a bug"
            ),
            Self::StateViolation => write!(f, "Parser encountered an unexpected or invalid state"),
            Self::InvalidChar => write!(f, "Parser encountered an invalid character"),
            Self::ScalarInvalid => write!(f, "Parser encountered an invalid scalar"),
            Self::EOFMapping => write!(
                f,
                "Parser encountered an unexpected EOF while parsing a mapping"
            ),
            Self::InvalidEOL => write!(f, "Parser encountered an invalid EOL"),
            Self::InvalidEOF => write!(f, "Parser encountered an invalid EOF"),
            Self::SoloCarriageReturn => write!(f, "Parser encountered a solo carriage return"),
        }
    }
}

#[derive(Debug)]
pub enum Context {
    Generic(Box<str>),
    BadChar(u8),
    ExpectedMultipleChar((Vec<u8>, u8)),
}

impl From<&str> for Context {
    fn from(s: &str) -> Self {
        Context::Generic(Box::from(s.as_ref()))
    }
}

impl From<u8> for Context {
    fn from(ch: u8) -> Self {
        Context::BadChar(ch)
    }
}

impl<T: AsRef<[u8]>> From<(T, u8)> for Context {
    fn from((good, bad): (T, u8)) -> Self {
        Context::ExpectedMultipleChar((Vec::from(good.as_ref()), bad))
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Generic(cxt) => write!(f, "{}", cxt),
            Self::BadChar(ch) => write!(f, "Bad char: '{}'", *ch as char),
            Self::ExpectedMultipleChar((good, bad)) => {
                if good.len() == 0 {
                    write!(f, "Bad char: '{}'", *bad as char)
                } else if good.len() == 1 {
                    write!(f, "Expected: '{}' got: '{}'", good[0] as char, *bad as char)
                } else {
                    write!(f, "Expected one of: [")?;
                    let len = good.len();
                    for (i, ch) in good.iter().enumerate() {
                        if i == len {
                            write!(f, "'{}'", *ch as char)?;
                        } else {
                            write!(f, "'{}', ", *ch as char)?;
                        }
                    }
                    write!(f, "] got: {}", *bad as char)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Io,
    State,
    Data,
}
