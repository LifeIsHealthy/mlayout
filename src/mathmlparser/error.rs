use std;
use std::fmt;
use std::io::prelude::*;

use super::quick_xml;
use super::quick_xml::XmlReader;

#[derive(Debug)]
pub struct ParsingError {
    pub position: Option<usize>,
    pub error_type: ErrorType,
}
impl ParsingError {
    pub fn from_string<B: BufRead>(parser: &XmlReader<B>, string: &str) -> ParsingError {
        ParsingError {
            position: Some(parser.buffer_position()),
            error_type: ErrorType::OtherError(string.into()),
        }
    }

    pub fn of_type<B: BufRead>(parser: &XmlReader<B>, err_type: ErrorType) -> ParsingError {
        ParsingError {
            position: Some(parser.buffer_position()),
            error_type: err_type,
        }
    }
}

#[derive(Debug)]
pub enum ErrorType {
    UnknownElement(String),
    UnexpectedEndOfInput,
    XmlError(quick_xml::error::Error),
    OtherError(String),
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.error_type {
            ErrorType::UnknownElement(ref name) => write!(f, "Unknown Element: \"{}\"", name),
            ErrorType::UnexpectedEndOfInput => write!(f, "Unexpected end of input."),
            ErrorType::OtherError(ref string) => write!(f, "Error: {}", string),
            ErrorType::XmlError(ref error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ParsingError {
    fn description(&self) -> &str {
        match self.error_type {
            ErrorType::UnknownElement(..) => "Encountered unknown element.",
            ErrorType::UnexpectedEndOfInput => "Unexpected end of input.",
            ErrorType::OtherError(ref msg) => msg,
            ErrorType::XmlError(_) => "Error while reading xml.",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match self.error_type {
            ErrorType::XmlError(ref error) => Some(error),
            _ => None,
        }
    }
}
impl<'a> ::std::convert::From<&'a str> for ParsingError {
    fn from(string: &str) -> ParsingError {
        ParsingError {
            position: None,
            error_type: ErrorType::OtherError(string.to_owned()),
        }
    }
}
impl ::std::convert::From<String> for ParsingError {
    fn from(string: String) -> ParsingError {
        ParsingError {
            position: None,
            error_type: ErrorType::OtherError(string),
        }
    }
}
impl ::std::convert::From<quick_xml::error::Error> for ParsingError {
    fn from(error: quick_xml::error::Error) -> ParsingError {
        ParsingError {
            position: None,
            error_type: ErrorType::XmlError(error),
        }
    }
}
impl ::std::convert::From<(quick_xml::error::Error, usize)> for ParsingError {
    fn from((error, position): (quick_xml::error::Error, usize)) -> ParsingError {
        ParsingError {
            position: Some(position),
            error_type: ErrorType::XmlError(error),
        }
    }
}
impl ::std::convert::From<std::str::Utf8Error> for ParsingError {
    fn from(error: std::str::Utf8Error) -> ParsingError {
        ParsingError {
            position: None,
            error_type: ErrorType::XmlError(quick_xml::error::Error::Utf8(error)),
        }
    }
}