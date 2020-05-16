use std;
use std::fmt;
use std::io::prelude::*;

#[cfg(feature = "mathml_parser")]
use quick_xml::{self, XmlReader};

pub type Result<T> = std::result::Result<T, ParsingError>;

#[derive(Debug)]
pub struct ParsingError {
    pub position: Option<usize>,
    pub error_type: ErrorType,
}
impl ParsingError {
    #[cfg(feature = "mathml_parser")]
    pub fn from_string<B: BufRead, S: ToString>(parser: &XmlReader<B>, string: S) -> ParsingError {
        ParsingError {
            position: Some(parser.buffer_position()),
            error_type: ErrorType::OtherError(string.to_string()),
        }
    }

    #[cfg(feature = "mathml_parser")]
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
    WrongEndElement(String),
    OtherError(String),
    Utf8Error(std::str::Utf8Error),
    #[cfg(feature = "mathml_parser")]
    XmlError(quick_xml::error::Error),
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.error_type {
            ErrorType::Utf8Error(err) => write!(f, "{}", err),
            ErrorType::UnknownElement(ref name) => write!(f, "Unknown Element: \"{}\"", name),
            ErrorType::UnexpectedEndOfInput => write!(f, "Unexpected end of input."),
            ErrorType::WrongEndElement(ref name) => write!(
                f,
                "Unexpected end element \"<{}>\" without corresponding start element.",
                name
            ),
            ErrorType::OtherError(ref string) => write!(f, "Error: {}", string),
            #[cfg(feature = "mathml_parser")]
            ErrorType::XmlError(ref error) => write!(f, "XML error: {}", error),
        }
    }
}
impl std::error::Error for ParsingError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self.error_type {
            #[cfg(feature = "mathml_parser")]
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
#[cfg(feature = "mathml_parser")]
impl ::std::convert::From<quick_xml::error::Error> for ParsingError {
    fn from(error: quick_xml::error::Error) -> ParsingError {
        ParsingError {
            position: None,
            error_type: ErrorType::XmlError(error),
        }
    }
}
#[cfg(feature = "mathml_parser")]
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
            error_type: ErrorType::Utf8Error(error),
        }
    }
}
