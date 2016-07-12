extern crate quick_xml;

use std::fmt;
use std::error::Error;
use std::io::BufRead;

use self::quick_xml::{XmlReader, Event, Element, AsStr};

use types::{List, ListItem, Field, Atom, AtomType, GeneralizedFraction};

#[derive(Debug)]
pub struct ParsingError {
    position: Option<usize>,
    error_type: ErrorType,
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
            ErrorType::UnknownElement(..) => "Element not known by the parser",
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

fn atom_type_for_mathml_element_name(name: &[u8]) -> AtomType {
    match name {
        b"mo" => AtomType::Bin,
        b"msqrt" => AtomType::Rad,
        _ => AtomType::Ord,
    }
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
                Some(Ok(Field::Unicode(string)))
            }
            Event::Start(elem) => {
                match elem.name() {
                    b"mglyph" | b"malignmark" => unimplemented!(),
                    _ => Some(Err(ParsingError::from_string(parser, "unexpected new element"))),
                }
            }
            Event::End(ref end_elem) => {
                if elem.name() == end_elem.name() {
                    break;
                }
                None
            }
            _ => Some(Err(ParsingError::from("Unknown Error"))),
        };
        if let Some(result) = result {
            fields.push(try!(result));
        }
    }
    if fields.is_empty() {
        return Err(ParsingError::from_string(parser, "empty token"));
    }
    Ok(fields)
}

fn parse_presentation_expression<R: BufRead>(parser: &mut XmlReader<R>,
                                             elem: &Element)
                                             -> Result<ListItem, ParsingError> {
    let name = elem.name();
    let advance = |parser: &mut XmlReader<R>| {
        let event = try!(parser.next()
            .ok_or(ParsingError::from_string(parser, "Unexpected end of input")));
        if let Event::Start(element) = try!(event) {
            Ok(element)
        } else {
            Err(ParsingError::from_string(parser, "Unexpected input"))
        }
    };
    match name {
        // Token expression
        b"mi" | b"mn" | b"mo" | b"msqrt" => {
            let mut fields = try!(parse_token(parser, elem));
            let field = if fields.len() == 1 {
                fields.remove(0)
            } else {
                let list = fields.into_iter()
                    .map(|field| ListItem::Atom(Atom::new_with_nucleus(AtomType::Ord, field)));
                Field::List(list.collect())
            };

            let atom_type = atom_type_for_mathml_element_name(name);
            let atom = Atom::new_with_nucleus(atom_type, field);
            Ok(ListItem::Atom(atom))
        }
        b"mfrac" => {
            let new_elem = try!(advance(parser));
            let numerator = try!(parse_presentation_expression(parser, &new_elem));
            let new_elem = try!(advance(parser));
            let denominator = try!(parse_presentation_expression(parser, &new_elem));

            match try!(parser.next()
                .ok_or(ParsingError::from_string(parser, "unexpected end of input"))) {
                Ok(Event::End(ref end_elem)) => {
                    if end_elem.name() != name {
                        return Err(ParsingError::from_string(parser, "unexpected end element"));
                    }
                }
                _ => return Err(ParsingError::from_string(parser, "unexpected event")),
            };

            let frac = GeneralizedFraction {
                numerator: numerator.into(),
                denominator: denominator.into(),
            };
            Ok(ListItem::GeneralizedFraction(frac))
        }
        _ => {
            Err(ParsingError {
                position: None,
                error_type: ErrorType::UnknownElement(try!(name.as_str()).into()),
            })
        }
    }
}

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
//
// #[allow(dead_code)]
// fn list_item_from_node<R: BufRead>(parser: &mut XmlReader<R>,
//                                    elem: &Element)
//                                    -> Result<ListItem, ParsingError> {
//     let name = elem.name();
//     match name {
//         b"mi" | b"mn" | b"mo" | b"msqrt" => {
//             let mut fields = try!(parse_token(parser, elem));
//             let atom_type = atom_type_for_mathml_element_name(name);
//             let atom = Atom::new_with_nucleus(atom_type, fields.remove(0));
//             Ok(ListItem::Atom(atom))
//         }
//         b"mfrac" => {
//             let numerator = try!(parse_token(parser, elem)).remove(0);
//             let denominator = try!(parse_token(parser, elem)).remove(0);
//             let frac = GeneralizedFraction {
//                 numerator: numerator,
//                 denominator: denominator,
//             };
//             Ok(ListItem::GeneralizedFraction(frac))
//         }
//         _ => {
//             Err(ParsingError {
//                 position: None,
//                 error_type: ErrorType::UnknownElement(try!(name.as_str()).into()),
//             })
//         }
//     }
// }
//
// // None if both an elemenet
// #[allow(unused_variables)]
// #[allow(dead_code)]
// fn find_children_elements<R: BufRead>(parser: &mut XmlReader<R>,
//                                       elem: &Element)
//                                       -> Result<Event, ParsingError> {
//     let mut tmp_event: Option<Event> = None;
//     loop {
//         let event = parser.next();
//         match event {
//             Some(Ok(event @ Event::Text(..))) => tmp_event = Some(event),
//             Some(Ok(event @ Event::Start(..))) => {
//                 match tmp_event {
//                     None => return Ok(event),
//                     _ => return Err(ParsingError::from_string(parser, "unexpected new element")),
//                 }
//             }
//             Some(Ok(Event::End(..))) => return Ok(tmp_event.unwrap()),
//             Some(Err(xml_error)) => return Err(xml_error.into()),
//             None => return Err(ParsingError::from_string(parser, "unexpected end of input")),
//             _ => {}
//         }
//     }
// }
//
// #[allow(dead_code)]
// fn field_from_node<R: BufRead>(parser: &mut XmlReader<R>,
//                                elem: &Element)
//                                -> Result<Field, ParsingError> {
//     let event = try!(find_children_elements(parser, elem));
//     match event {
//         Event::Start(..) => list_from_node(parser, &event).map(Field::List),
//         Event::Text(ident) => {
//             let text_bytes = try!(ident.unescaped_content()).into_owned();
//             let string = try!(String::from_utf8(text_bytes)
//                 .map_err(|x| quick_xml::error::Error::Utf8(x.utf8_error())));
//             Ok(Field::Unicode(string))
//         }
//         _ => Err("internal error: invalid field".into()),
//     }
// }
//
// #[allow(match_same_arms)]
// #[allow(dead_code)]
// fn list_from_node<R: BufRead>(parser: &mut XmlReader<R>,
//                               event: &Event)
//                               -> Result<List, ParsingError> {
//     let mut list = List::new();
//     match *event {
//         Event::Start(ref elem) => try!(list_item_from_node(parser, elem).map(|x| list.push(x))),
//         Event::End(..) => return Err("blabla".into()),
//         _ => {}
//     }
//     loop {
//         let event = parser.find(|x| match *x {
//             Ok(Event::End(..)) |
//             Ok(Event::Start(..)) => true,
//             _ => false,
//         });
//         match event {
//             Some(Ok(Event::Start(ref elem))) => {
//                 try!(list_item_from_node(parser, elem).map(|x| list.push(x)))
//             }
//             Some(Ok(Event::End(..))) => break,
//             None => break,
//             _ => {}
//         }
//     }
//     Ok(list)
// }
