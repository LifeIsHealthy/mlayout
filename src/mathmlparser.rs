extern crate xml;

use std::fmt;
use std::error::Error;
use std::io::Read;

use std::iter::Peekable;

use self::xml::reader::{EventReader, XmlEvent, Events};

use types::{List, ListItem, Field, Atom, AtomType};

type PeekableEvents<R> = Peekable<Events<R>>;

trait PeekFind: Iterator {
    fn peek_find<P>(&mut self, mut predicate: P)
        where Self: Sized,
              P: FnMut(&Self::Item) -> bool;
}
impl<T: Iterator> PeekFind for Peekable<T> {
    #[allow(while_let_loop)]
    fn peek_find<P>(&mut self, mut predicate: P)
        where Self: Sized,
              P: FnMut(&Self::Item) -> bool
    {
        loop {
            match self.peek() {
                Some(x) => {
                    if predicate(x) {
                        break;
                    }
                }
                None => break,
            }
            self.next();
        }
    }
}

#[derive(Debug)]
pub enum ParsingError {
    UnknownElement(String),
    OtherError(String),
}
impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParsingError::UnknownElement(ref name) => write!(f, "Unknown Element: \"{}\"", name),
            ParsingError::OtherError(ref string) => write!(f, "Error: {}", string),
        }
    }
}
impl Error for ParsingError {
    fn description(&self) -> &str {
        match *self {
            ParsingError::UnknownElement(..) => "Element not known by the parser.",
            ParsingError::OtherError(ref msg) => msg,
        }
    }
}
impl<T: Into<String>> ::std::convert::From<T> for ParsingError {
    fn from(string: T) -> ParsingError {
        ParsingError::OtherError(string.into())
    }
}

pub fn parse_file<R: Read>(file: R) -> Result<List, ParsingError> {
    let parser = EventReader::new(file);
    let mut parser: PeekableEvents<R> = parser.into_iter().peekable();
    list_from_node(&mut parser)
}

fn atom_type_for_mathml_element_name(name: &str) -> AtomType {
    match name {
        "mo" => AtomType::Bin,
        "msqrt" => AtomType::Rad,
        _ => AtomType::Ord,
    }
}

fn list_item_from_node<R: Read>(parser: &mut PeekableEvents<R>) -> Result<ListItem, ParsingError> {
    let element = try!(parser.next().expect("internal error").map_err(|x| x.msg().to_owned()));
    let name = match element {
        XmlEvent::StartElement { ref name, .. } => &name.local_name,
        _ => panic!(),
    };
    match name.as_ref() {
        "mi" | "mn" | "mo" | "msqrt" => {
            let field = try!(field_from_node(parser).ok_or("invalid field"));
            let atom_type = atom_type_for_mathml_element_name(name);
            let atom = Atom::new_with_nucleus(atom_type, field);
            Ok(ListItem::Atom(atom))
        }
        _ => Err(ParsingError::UnknownElement(name.clone())),
    }
}

enum ElementOrText {
    Element(XmlEvent),
    Text(String),
    None,
}

// None if both an elemenet
fn find_children_elements<R: Read>(parser: &mut PeekableEvents<R>) -> ElementOrText {
    let mut element_or_text = ElementOrText::None;
    loop {
        let elem = parser.next();
        match elem {
            Some(Ok(XmlEvent::Characters(identifier))) => {
                element_or_text = ElementOrText::Text(identifier)
            }
            Some(Ok(event @ XmlEvent::StartElement { .. })) => {
                match element_or_text {
                    ElementOrText::None => return ElementOrText::Element(event),
                    _ => return ElementOrText::None,
                }
            }
            Some(Ok(XmlEvent::EndElement { .. })) => return element_or_text,
            None => return ElementOrText::None,
            _ => {}
        }
    }
}

fn field_from_node<R: Read>(mut parser: &mut PeekableEvents<R>) -> Option<Field> {
    let element_or_text = find_children_elements(parser);

    match element_or_text {
        ElementOrText::Element(..) => list_from_node(&mut parser).map(Field::List).ok(),
        ElementOrText::Text(ident) => Some(Field::Unicode(ident)),
        _ => None,
    }
}

#[allow(match_same_arms)]
fn list_from_node<R: Read>(mut parser: &mut PeekableEvents<R>) -> Result<List, ParsingError> {
    let mut list = List::new();
    loop {
        let mut found_start_element = false;
        parser.peek_find(|x| match *x {
            Ok(XmlEvent::EndElement{ .. }) => true,
            Ok(XmlEvent::StartElement{ .. }) => {found_start_element = true; true},
            _ => false,
        });
        if found_start_element {
            try!(list_item_from_node(&mut parser).map(|x| list.push(x)));
        } else {
            break;
        }
    }
    Ok(list)
}
