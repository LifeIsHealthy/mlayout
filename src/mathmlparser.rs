extern crate xml;

use std::error::Error;
use std::io::{Read};

use self::xml::reader::{EventReader, XmlEvent, Events};

use types::{List, ListItem, Field, Atom, AtomType};

pub fn parse_file<'a, R: Read>(file: R) -> Result<List<'a>, Box<Error + Send + Sync>> {
    let parser = EventReader::new(file);

    let mut parser = parser.into_iter();

    let mut list = List::new();
    loop {
        let elem = parser.find(|x| match *x {
                Ok(XmlEvent::StartElement{ .. }) => true,
                _ => false
        });
        match elem {
            Some(Ok(element)) => {
                let item = list_item_from_node(&mut parser, &element);
                match item {
                    Some(item) => list.push(item),
                    None => return Err(From::from("List item invalid")),
                }
            },
            Some(Err(the_error)) => return Err(Box::new(the_error)),
            _ => break
        };
    }

    Ok(list)
    // Err(From::from("No Results"))
}

fn atom_type_for_mathml_element_name(name: &str) -> AtomType {
    match name {
        "mo" => AtomType::Bin,
        "msqrt" => AtomType::Rad,
        _ => AtomType::Ord,
    }
}

fn list_item_from_node<'a, R: Read>(parser: &mut Events<R>, element: &XmlEvent) -> Option<ListItem<'a>> {
    let name = match *element {
         XmlEvent::StartElement{ref name, ..} => &name.local_name,
        _ => panic!()
    };
    match name.as_ref() {
        "mi" | "mn" | "mo" | "msqrt" => {
            let field = field_from_node(parser);
            match field {
                Some(nucleus) => {
                    let atom_type = atom_type_for_mathml_element_name(name);
                    let atom = Atom::new_with_nucleus(atom_type, nucleus);
                    Some(ListItem::Atom(atom))
                },
                None => None
            }

        },
        _ => None
    }
}

enum EventOrText<'a> {
    Event(&'a XmlEvent),
    Text(String),
    None
}
fn field_from_node<'a, R: Read>(parser: &mut Events<R>) -> Option<Field<'a>> {
    let mut event_or_text = EventOrText::None;
    for element in parser {
        match element {
            Ok(XmlEvent::Characters(identifier)) => event_or_text = EventOrText::Text(identifier),
            Ok(ref event @ XmlEvent::StartElement{..}) => {event_or_text = EventOrText::Event(event); break},
            _ => {},
        }
    }
    match event_or_text {
        EventOrText::Event(elem) => {
            let list = List::new();
            let item = list_item_from_node(&mut parser, &elem);
            match item {
                Some(item) => list.push(item),
                None => return None,
            }
            loop {
                let elem = parser.find(|x| match *x {
                        Ok(XmlEvent::StartElement{ .. }) => true,
                        Ok(XmlEvent::EndElement{..}) => true,
                        _ => false
                });
                match elem {
                    Some(Ok(XmlEvent::EndElement{..})) => break,
                    Some(Ok(element)) => {
                        let item = list_item_from_node(&mut parser, &element);
                        match item {
                            Some(item) => list.push(item),
                            None => return None,
                        }
                    },
                    Some(Err(the_error)) => return None,
                    _ => break,
                };
            }
            Some(Field::List(list))
        },
        EventOrText::Text(ident) => Some(Field::Unicode(ident)),
    }
}

pub fn hello_world() {
    println!("Hello World!");
}
