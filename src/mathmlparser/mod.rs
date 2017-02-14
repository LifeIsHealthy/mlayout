extern crate quick_xml;

mod error;
mod operator;
mod operator_dict;
mod token;
mod escape;

use std;
use std::io::BufRead;
use std::borrow::Cow;

use types::{Atom, OverUnder, GeneralizedFraction, Root, Length, MathExpression, MathItem};

pub use self::quick_xml::{XmlReader, Event, Element};
pub use self::quick_xml::error::ResultPos;
use self::operator::{guess_if_operator_with_form, Form};

pub use self::error::*;

pub type MExpression = MathExpression<MathmlInfo>;
type Result<T> = std::result::Result<T, ParsingError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathmlElement {
    identifier: &'static str,
    elem_type: ElementType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ElementType {
    TokenElement,
    LayoutSchema { args: ArgumentRequirements },
    MathmlRoot,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgumentRequirements {
    ArgumentList, // single argument or inferred mrow
    RequiredArguments(u8), // the number of required arguments
    Special,
}

pub trait FromXmlAttribute: Sized {
    type Err;
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err>;
}

pub trait AttributeParse {
    fn parse_xml<T: FromXmlAttribute>(&self) -> std::result::Result<T, T::Err>;
}

impl AttributeParse for [u8] {
    fn parse_xml<T: FromXmlAttribute>(&self) -> std::result::Result<T, T::Err> {
        <T as FromXmlAttribute>::from_xml_attr(self)
    }
}

// a static list of all mathml elements
static MATHML_ELEMENTS: [MathmlElement; 15] =
    [MathmlElement {
         identifier: "mi",
         elem_type: ElementType::TokenElement,
     },
     MathmlElement {
         identifier: "mo",
         elem_type: ElementType::TokenElement,
     },
     MathmlElement {
         identifier: "mn",
         elem_type: ElementType::TokenElement,
     },
     MathmlElement {
         identifier: "mtext",
         elem_type: ElementType::TokenElement,
     },
     MathmlElement {
         identifier: "mrow",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::ArgumentList },
     },
     MathmlElement {
         identifier: "math",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::ArgumentList },
     },
     MathmlElement {
         identifier: "msub",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     },
     MathmlElement {
         identifier: "msup",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     },
     MathmlElement {
         identifier: "msubsup",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(3) },
     },
     MathmlElement {
         identifier: "mover",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     },
     MathmlElement {
         identifier: "munder",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     },
     MathmlElement {
         identifier: "munderover",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(3) },
     },
     MathmlElement {
         identifier: "mfrac",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     },
     MathmlElement {
         identifier: "msqrt",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::ArgumentList },
     },
     MathmlElement {
         identifier: "mroot",
         elem_type: ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(2) },
     }];

fn match_math_element(identifier: &[u8]) -> Option<MathmlElement> {
    for elem in MATHML_ELEMENTS.iter() {
        if elem.identifier.as_bytes() == identifier {
            return Some(*elem);
        }
    }
    None
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MathmlInfo {
    operator_attrs: Option<operator::Attributes>,
    pub is_space: bool,
}

impl MathmlInfo {
    fn is_operator(&self) -> bool {
        !self.operator_attrs.is_none()
    }
}

pub fn parse<R: BufRead>(file: R) -> Result<MExpression> {
    let mut parser = XmlReader::from_reader(file).trim_text(true);
    let root_elem = MathmlElement {
        identifier: "ROOT_ELEMENT", // this identifier is arbitrary and should not be used
        elem_type: ElementType::MathmlRoot,
    };
    parse_element(&mut parser, root_elem, std::iter::empty())
}

fn parse_element<'a, R: BufRead, A>(parser: &mut XmlReader<R>,
                                    elem: MathmlElement,
                                    attributes: A)
                                    -> Result<MExpression>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    match elem.elem_type {
        ElementType::TokenElement => token::parse(parser, elem, attributes),
        ElementType::LayoutSchema { args: ArgumentRequirements::ArgumentList } |
        ElementType::MathmlRoot => {
            let mut list = parse_element_list(parser, elem)?;
            operator::process_operators(&mut list);
            parse_list_schema(list, elem, attributes)
        }
        ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(_) } => {
            let arguments = parse_fixed_arguments(parser, elem);
            parse_fixed_schema(arguments?, elem, attributes)
        }
        _ => unimplemented!(),
    }
}

fn parse_sub_element<R: BufRead>(parser: &mut XmlReader<R>, elem: &Element) -> Result<MExpression> {
    let sub_elem = match_math_element(elem.name());
    match sub_elem {
        Some(sub_elem) => parse_element(parser, sub_elem, elem.attributes()),
        None => {
            let name = String::from_utf8_lossy(elem.name()).into_owned();
            let result: Result<_> = parser.read_to_end(elem.name()).map_err(|err| err.into());
            result.and(Err(ParsingError::of_type(parser, ErrorType::UnknownElement(name))))
        }
    }
}

fn parse_element_list<R: BufRead>(parser: &mut XmlReader<R>,
                                  elem: MathmlElement)
                                  -> Result<Vec<MExpression>> {
    let mut list = Vec::new();
    loop {
        let next_event = parser.next();
        match next_event {
            Some(Ok(Event::Start(ref start_elem))) => {
                list.push(parse_sub_element(parser, start_elem)?)
            }
            Some(Ok(Event::End(ref end_elem))) => {
                if elem.elem_type == ElementType::MathmlRoot {
                    let name = std::str::from_utf8(end_elem.name())?.to_string();
                    return Err(ParsingError::of_type(parser, ErrorType::WrongEndElement(name)));
                }
                if end_elem.name() == elem.identifier.as_bytes() {
                    break;
                } else {
                    let name = std::str::from_utf8(end_elem.name())?.to_string();
                    return Err(ParsingError::of_type(parser, ErrorType::WrongEndElement(name)));
                }
            }
            Some(Err(error)) => Err(error)?,
            None => {
                if elem.elem_type == ElementType::MathmlRoot {
                    break;
                } else {
                    return Err(ParsingError::of_type(parser, ErrorType::UnexpectedEndOfInput));
                }
            }
            _ => {}
        }
    }
    Ok(list)
}

fn parse_list_schema<'a, A>(content: Vec<MExpression>,
                            elem: MathmlElement,
                            _: A)
                            -> Result<MExpression>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    // a mrow with a single element is strictly equivalent to the element
    let content = if content.len() == 1 {
        content.into_iter().next().unwrap()
    } else {
        MExpression {
            content: MathItem::List(content),
            user_info: Default::default(),
        }
    };
    if elem.elem_type == ElementType::MathmlRoot {
        return Ok(content);
    }
    match elem.identifier {
        "mrow" | "math" => Ok(content),
        "msqrt" => {
            let item = MathItem::Root(Box::new(Root { radicand: content, ..Default::default() }));
            Ok(MathExpression {
                content: item,
                user_info: Default::default(),
            })
        }
        _ => unimplemented!(),
    }
}

fn parse_fixed_arguments<'a, R: BufRead>(parser: &mut XmlReader<R>,
                                         elem: MathmlElement)
                                         -> Result<Vec<MExpression>> {
    if let ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(num_args) } =
        elem.elem_type {
        let args = parse_element_list(parser, elem)?;
        if args.len() == num_args as usize {
            Ok(args)
        } else {
            Err(ParsingError::from_string(parser,
                                          format!("\"{:?}\" element requires {:?} arguments. \
                                                   Found {:?} arguments.",
                                                  elem.identifier,
                                                  num_args,
                                                  args.len())))
        }
    } else {
        unreachable!();
    }
}

fn parse_fixed_schema<'a, A>(mut content: Vec<MExpression>,
                             elem: MathmlElement,
                             attributes: A)
                             -> Result<MExpression>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    let mut arguments = content.into_iter();
    let result = match elem.identifier {
        "mfrac" => {
            let frac = GeneralizedFraction {
                numerator: arguments.next().unwrap(),
                denominator: arguments.next().unwrap(),
                thickness: None,
            };
            MathItem::GeneralizedFraction(Box::new(frac))
        }
        "mroot" => {
            let root = Root {
                radicand: arguments.next().unwrap(),
                degree: arguments.next().unwrap(),
            };
            MathItem::Root(Box::new(root))
        }
        "msub" => {
            let atom = Atom {
                nucleus: arguments.next().unwrap(),
                bottom_right: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                ..Default::default()
            };
            MathItem::Atom(Box::new(atom))
        }
        "msup" => {
            let atom = Atom {
                nucleus: arguments.next().unwrap(),
                top_right: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                ..Default::default()
            };
            MathItem::Atom(Box::new(atom))
        }
        "msubsup" => {
            let atom = Atom {
                nucleus: arguments.next().unwrap(),
                bottom_right: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                top_right: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                ..Default::default()
            };
            MathItem::Atom(Box::new(atom))
        }
        "mover" => {
            let mut as_accent = false;
            for attrib in attributes {
                let (ident, value) = attrib?;
                if ident == b"accent" && &value as &[u8] == b"true" {
                    as_accent = true;
                }
            }
            let item = OverUnder {
                nucleus: arguments.next().unwrap(),
                over: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                over_is_accent: as_accent,
                ..Default::default()
            };
            MathItem::OverUnder(Box::new(item))
        }
        "munder" => {
            let item = OverUnder {
                nucleus: arguments.next().unwrap(),
                under: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                ..Default::default()
            };
            MathItem::OverUnder(Box::new(item))
        }
        "munderover" => {
            let item = OverUnder {
                nucleus: arguments.next().unwrap(),
                under: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                over: guess_if_operator_with_form(arguments.next().unwrap(), Form::Postfix),
                ..Default::default()
            };
            MathItem::OverUnder(Box::new(item))
        }
        _ => unreachable!(),
    };
    let info = MathmlInfo {
        operator_attrs: match result {
            MathItem::Atom(ref atom) => atom.nucleus.user_info.operator_attrs,
            MathItem::OverUnder(ref ou) => ou.nucleus.user_info.operator_attrs,
            MathItem::GeneralizedFraction(ref frac) => frac.numerator.user_info.operator_attrs,
            _ => None,
        },
        ..Default::default()
    };
    Ok(MExpression {
        content: result,
        user_info: info,
    })
}


impl FromXmlAttribute for Length {
    type Err = ParsingError;
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err> {
        unimplemented!()
    }
}

impl FromXmlAttribute for bool {
    type Err = ParsingError;
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err> {
        match bytes {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(ParsingError::from("unrecognized boolean value")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::operator::Form;
    use types::*;

    fn find_operator(expr: MExpression) -> MExpression {
        match expr.content {
            MathItem::List(list) => {
                list.into_iter()
                    .filter(|expr| expr.user_info.is_operator())
                    .next()
                    .expect("List contains no operator.")
            }
            MathItem::Operator(_) => expr,
            other_item => panic!("Expected list or Operator. Found {:?}", other_item),
        }
    }

    #[test]
    fn test_operator() {
        let xml = "<mo>+</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(expr);
        assert!(operator.user_info.is_operator());
        match operator.content {
            MathItem::Operator(Operator { field: Field::Unicode(text), .. }) => {
                assert_eq!(text, "+")
            }
            other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_prefix_operator() {
        let xml = "<mo>-</mo><mi>x</mi>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(expr);
        assert_eq!(operator.user_info.operator_attrs.unwrap().form.unwrap(),
                   Form::Prefix);
        match operator.content {
            MathItem::Operator(Operator { field: Field::Unicode(text), .. }) => {
                assert_eq!(text, "\u{2212}") // MINUS SIGN
            }
            other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_infix_operator() {
        let xml = "<mi>x</mi><mo>=</mo><mi>y</mi>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(expr);
        assert_eq!(operator.user_info.operator_attrs.unwrap().form.unwrap(),
                   Form::Infix);
        match operator.content {
            MathItem::Operator(Operator { field: Field::Unicode(text), .. }) => {
                assert_eq!(text, "=")
            }
            other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_postfix_operator() {
        let xml = "<mi>x</mi><mo>!</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(expr);
        assert_eq!(operator.user_info.operator_attrs.unwrap().form.unwrap(),
                   Form::Postfix);
        match operator.content {
            MathItem::Operator(Operator { field: Field::Unicode(text), .. }) => {
                assert_eq!(text, "!")
            }
            other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }
}
