extern crate quick_xml;

use std::fmt;
use std::error::Error;
use std::io::BufRead;

use self::quick_xml::{XmlReader, Event, Element, AsStr};

use types::{List, ListItem, Field, Atom, OverUnder, GeneralizedFraction};
use types::Field::Empty;

use unicode_math::{Family, convert_character_to_family};

#[derive(Debug)]
pub struct ParsingError {
    pub position: Option<usize>,
    pub error_type: ErrorType,
}
impl ParsingError {
    fn from_string<B: BufRead>(parser: &XmlReader<B>, string: &str) -> ParsingError {
        ParsingError {
            position: Some(parser.buffer_position()),
            error_type: ErrorType::OtherError(string.into()),
        }
    }
}

#[derive(Debug)]
pub enum ErrorType {
    UnknownElement(String),
    XmlError(quick_xml::error::Error),
    OtherError(String),
}
impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.error_type {
            ErrorType::UnknownElement(ref name) => write!(f, "Unknown Element: \"{}\"", name),
            ErrorType::OtherError(ref string) => write!(f, "Error: {}", string),
            ErrorType::XmlError(ref error) => error.fmt(f),
        }
    }
}
impl Error for ParsingError {
    fn description(&self) -> &str {
        match self.error_type {
            ErrorType::UnknownElement(..) => "Element is not known to the parser.",
            ErrorType::OtherError(ref msg) => msg,
            ErrorType::XmlError(ref error) => error.description(),
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

pub fn parse<R: BufRead>(file: R) -> Result<List, ParsingError> {
    let mut parser = XmlReader::from_reader(file).trim_text(true);
    parse_math_list(&mut parser, None)
}

// invoked after a token expression
// the cursor is moved behind the end element of the token expression
// the result (if ok) is guaranteed to not be empty
fn parse_token<R: BufRead>(parser: &mut XmlReader<R>,
                           elem: &Element)
                           -> Result<Vec<Field>, ParsingError> {
    let mut fields: Vec<Field> = Vec::new();
    while let Some(event) = parser.next() {
        let result = match try!(event) {
            Event::Text(text) => {
                let text_bytes = try!(text.unescaped_content()).into_owned();
                let string = try!(String::from_utf8(text_bytes)
                    .map_err(|x| quick_xml::error::Error::Utf8(x.utf8_error())));
                Ok(Field::Unicode(string))
            }
            Event::Start(elem) => {
                match elem.name() {
                    b"mglyph" | b"malignmark" => unimplemented!(),
                    _ => Err(ParsingError::from_string(parser, "unexpected new element")),
                }
            }
            Event::End(ref end_elem) => {
                if elem.name() == end_elem.name() {
                    break;
                }
                continue;
            }
            _ => Err(ParsingError::from("Unknown Error")),
        };
        fields.push(try!(result));
    }

    if fields.is_empty() {
        Err(ParsingError::from_string(parser, "empty token"))
    } else {
        Ok(fields)
    }
}

fn check_start_element<R: BufRead>(parser: &mut XmlReader<R>) -> Result<Element, ParsingError> {
    let event = try!(parser.next()
        .ok_or(ParsingError::from_string(parser, "Unexpected end of input")));
    if let Event::Start(element) = try!(event) {
        Ok(element)
    } else {
        Err(ParsingError::from_string(parser, "Unexpected input"))
    }
}

fn check_end_element<R: BufRead>(parser: &mut XmlReader<R>,
                                 name: &[u8])
                                 -> Result<(), ParsingError> {
    match try!(parser.next()
        .ok_or(ParsingError::from_string(parser, "unexpected end of input"))) {
        Ok(Event::End(ref end_elem)) => {
            if end_elem.name() != name {
                Err(ParsingError::from_string(parser, "unexpected end element"))
            } else {
                Ok(())
            }
        }
        Ok(event) => {
            let msg = format!("unexpected event {:?}", event);
            Err(ParsingError::from_string(parser, &msg))
        }
        Err(error) => Err(error.into()),
    }
}

fn assume_presentation_expression<R: BufRead>(parser: &mut XmlReader<R>)
                                              -> Result<ListItem, ParsingError> {
    let start_elem = try!(check_start_element(parser));
    parse_presentation_expression(parser, &start_elem)
}

fn parse_presentation_expression<R: BufRead>(parser: &mut XmlReader<R>,
                                             elem: &Element)
                                             -> Result<ListItem, ParsingError> {
    let name = elem.name();
    match name {
        // Token expression
        b"mi" | b"mn" | b"mo" | b"msqrt" => {
            let mut fields = try!(parse_token(parser, elem));
            let field = if fields.len() == 1 {
                let field = fields.remove(0);
                match (name, field) {
                    (b"mi", Field::Unicode(ref text)) if text.len() == 1 => {
                        Field::Unicode(convert_character_to_family(text.chars().next().unwrap(),
                                                                   Family::Italics)
                            .to_string())
                    }
                    (_, field) => field,
                }
            } else {
                let list = fields.into_iter()
                    .map(|field| ListItem::Atom(Atom::new_with_nucleus(field)));
                Field::List(list.collect())
            };

            let atom = Atom::new_with_nucleus(field);
            Ok(ListItem::Atom(atom))
        }
        b"mfrac" => {
            let numerator: Field = try!(assume_presentation_expression(parser)).into();
            let denominator: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let frac = GeneralizedFraction {
                numerator: numerator,
                denominator: denominator,
            };
            Ok(ListItem::GeneralizedFraction(frac))
        }
        b"mrow" => {
            let list = try!(parse_math_list(parser, Some(&elem)));
            Ok(ListItem::Atom(Atom::new_with_nucleus(Field::List(list))))
        }
        b"msub" => {
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let subscript: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let atom = Atom::new_with_attachments(nucleus, Empty, Empty, Empty, subscript);
            Ok(ListItem::Atom(atom))
        }
        b"msup" => {
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let superscript: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let atom = Atom::new_with_attachments(nucleus, Empty, superscript, Empty, Empty);
            Ok(ListItem::Atom(atom))
        }
        b"msubsup" => {
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let subscript: Field = try!(assume_presentation_expression(parser)).into();
            let superscript: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let atom = Atom::new_with_attachments(nucleus, Empty, superscript, Empty, subscript);
            Ok(ListItem::Atom(atom))
        }
        b"mover" => {
            let attributes = elem.unescaped_attributes();
            let mut as_accent = false;
            for attrib in attributes {
                let (ident, value) = try!(attrib);
                if ident == b"accent" && &value as &[u8] == b"true" {
                    as_accent = true;
                }
            }
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let over: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let item = OverUnder { nucleus: nucleus, over: over, over_is_accent: as_accent, ..Default::default() };
            Ok(ListItem::OverUnder(item))
        }
        b"munder" => {
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let under: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let item = OverUnder { nucleus: nucleus, under: under, ..Default::default() };
            Ok(ListItem::OverUnder(item))
        }
        b"munderover" => {
            let nucleus: Field = try!(assume_presentation_expression(parser)).into();
            let under: Field = try!(assume_presentation_expression(parser)).into();
            let over: Field = try!(assume_presentation_expression(parser)).into();
            try!(check_end_element(parser, name));

            let item = OverUnder { nucleus: nucleus, under: under, over: over, ..Default::default() };
            Ok(ListItem::OverUnder(item))
        }
        _ => {
            Err(ParsingError {
                position: None,
                error_type: ErrorType::UnknownElement(try!(name.as_str()).into()),
            })
        }
    }
}

// <elem> |cursor at start|
//      <list_begin />
//      ...
//      <list_end />
// <elem/>
// |cursor at end|
fn parse_math_list<R: BufRead>(parser: &mut XmlReader<R>,
                               elem: Option<&Element>)
                               -> Result<List, ParsingError> {
    let mut list = List::new();
    loop {
        let event = parser.next();
        match event {
            Some(Ok(Event::Start(ref start_elem))) => {
                try!(parse_presentation_expression(parser, start_elem).map(|x| list.push(x)))
            }
            Some(Ok(Event::End(ref end_elem))) => {
                if let Some(elem) = elem {
                    if elem.name() == end_elem.name() {
                        break;
                    }
                }
                return Err(ParsingError::from_string(parser, "unexpected end element"));
            }
            None => {
                if let Some(..) = elem {
                    return Err(ParsingError::from_string(parser, "unexpected end of input"));
                } else {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(list)
}
