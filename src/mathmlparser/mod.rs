extern crate quick_xml;
extern crate vec_map;

mod error;
mod operator;
mod operator_dict;
mod token;
mod escape;

use std;
use std::io::BufRead;

use types::{Atom, OverUnder, GeneralizedFraction, Root, Length, MathExpression, MathItem, Index};

pub use self::quick_xml::{XmlReader, Event, Element};
pub use self::quick_xml::error::ResultPos;

use self::vec_map::VecMap;

use self::operator::{guess_if_operator_with_form, Form};

pub use self::error::*;

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

pub struct ParseContext {
    pub expr: MathExpression,
    pub mathml_info: VecMap<MathmlInfo>,
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

pub fn parse<R: BufRead>(file: R) -> Result<MathExpression> {
    let mut parser = XmlReader::from_reader(file).trim_text(true);
    let root_elem = MathmlElement {
        identifier: "ROOT_ELEMENT", // this identifier is arbitrary and should not be used
        elem_type: ElementType::MathmlRoot,
    };
    let expr = MathExpression::new();
    let info = VecMap::new();
    let mut context = ParseContext {
        expr: expr,
        mathml_info: info,
    };

    match parse_element(&mut parser, root_elem, std::iter::empty(), &mut context) {
        Ok(index) => {
            context.expr.root_index = index;
            Ok(context.expr)
        }
        Err(err) => Err(err),
    }
}

fn parse_element<'a, R: BufRead, A>(parser: &mut XmlReader<R>,
                                    elem: MathmlElement,
                                    attributes: A,
                                    context: &mut ParseContext)
                                    -> Result<Index>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    match elem.elem_type {
        ElementType::TokenElement => token::parse(parser, elem, attributes, context),
        ElementType::LayoutSchema { args: ArgumentRequirements::ArgumentList } |
        ElementType::MathmlRoot => {
            let mut list = parse_element_list(parser, elem, context)?;
            operator::process_operators(&mut list, context);
            parse_list_schema(list, elem, attributes, context)
        }
        ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(_) } => {
            let arguments = parse_fixed_arguments(parser, elem, context);
            parse_fixed_schema(arguments?, elem, attributes, context)
        }
        _ => unimplemented!(),
    }
}

fn parse_sub_element<R: BufRead>(parser: &mut XmlReader<R>,
                                 elem: &Element,
                                 context: &mut ParseContext)
                                 -> Result<Index> {
    let sub_elem = match_math_element(elem.name());
    match sub_elem {
        Some(sub_elem) => parse_element(parser, sub_elem, elem.attributes(), context),
        None => {
            let name = String::from_utf8_lossy(elem.name()).into_owned();
            let result: Result<_> = parser.read_to_end(elem.name()).map_err(|err| err.into());
            result.and(Err(ParsingError::of_type(parser, ErrorType::UnknownElement(name))))
        }
    }
}

fn parse_element_list<R: BufRead>(parser: &mut XmlReader<R>,
                                  elem: MathmlElement,
                                  context: &mut ParseContext)
                                  -> Result<Vec<Index>> {
    let mut list = Vec::new();
    loop {
        let next_event = parser.next();
        match next_event {
            Some(Ok(Event::Start(ref start_elem))) => {
                list.push(parse_sub_element(parser, start_elem, context)?)
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

fn parse_list_schema<'a, A>(content: Vec<Index>,
                            elem: MathmlElement,
                            _: A,
                            context: &mut ParseContext)
                            -> Result<Index>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    // a mrow with a single element is strictly equivalent to the element
    let content = if content.len() == 1 {
        content[0]
    } else {
        let item = MathItem::List(content);
        context.expr.add_item(item)
    };
    if elem.elem_type == ElementType::MathmlRoot {
        return Ok(content);
    }
    match elem.identifier {
        "mrow" | "math" => Ok(content),
        "msqrt" => {
            let item = MathItem::Root(Root {
                                          radicand: content,
                                          ..Default::default()
                                      });
            Ok(context.expr.add_item(item))
        }
        _ => unimplemented!(),
    }
}

fn parse_fixed_arguments<'a, R: BufRead>(parser: &mut XmlReader<R>,
                                         elem: MathmlElement,
                                         context: &mut ParseContext)
                                         -> Result<Vec<Index>> {
    if let ElementType::LayoutSchema { args: ArgumentRequirements::RequiredArguments(num_args) } =
        elem.elem_type {
        let args = parse_element_list(parser, elem, context)?;
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

fn parse_fixed_schema<'a, A>(content: Vec<Index>,
                             elem: MathmlElement,
                             attributes: A,
                             context: &mut ParseContext)
                             -> Result<Index>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    let result = match elem.identifier {
        "mfrac" => {
            let frac = GeneralizedFraction {
                numerator: content[0],
                denominator: content[1],
                thickness: None,
            };
            MathItem::GeneralizedFraction(frac)
        }
        "mroot" => {
            let root = Root {
                radicand: content[0],
                degree: content[1],
            };
            MathItem::Root(root)
        }
        "msub" => {
            let atom = Atom {
                nucleus: content[0],
                bottom_right: guess_if_operator_with_form(content[1], Form::Postfix, context),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msup" => {
            let atom = Atom {
                nucleus: content[0],
                top_right: guess_if_operator_with_form(content[1], Form::Postfix, context),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msubsup" => {
            let atom = Atom {
                nucleus: content[0],
                bottom_right: guess_if_operator_with_form(content[1], Form::Postfix, context),
                top_right: guess_if_operator_with_form(content[2], Form::Postfix, context),
                ..Default::default()
            };
            MathItem::Atom(atom)
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
                nucleus: content[0],
                over: guess_if_operator_with_form(content[1], Form::Postfix, context),
                over_is_accent: as_accent,
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        "munder" => {
            let item = OverUnder {
                nucleus: content[0],
                under: guess_if_operator_with_form(content[1], Form::Postfix, context),
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        "munderover" => {
            let item = OverUnder {
                nucleus: content[0],
                under: guess_if_operator_with_form(content[1], Form::Postfix, context),
                over: guess_if_operator_with_form(content[2], Form::Postfix, context),
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        _ => unreachable!(),
    };
    let info = MathmlInfo {
        operator_attrs: match result {
            MathItem::Atom(ref atom) => {
                context.mathml_info.get(atom.nucleus.into()).and_then(|info| info.operator_attrs)
            }
            MathItem::OverUnder(ref ou) => {
                context.mathml_info.get(ou.nucleus.into()).and_then(|info| info.operator_attrs)
            }
            MathItem::GeneralizedFraction(ref frac) => {
                context.mathml_info.get(frac.numerator.into()).and_then(|info| info.operator_attrs)
            }
            _ => None,
        },
        ..Default::default()
    };
    let index = context.expr.add_item(result);
    context.mathml_info.insert(index.into(), info);
    Ok(index)
}


impl FromXmlAttribute for Length {
    type Err = ParsingError;
    fn from_xml_attr(_: &[u8]) -> std::result::Result<Self, Self::Err> {
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
    use types::*;

    fn find_operator(expr: &MathExpression) -> Index {
        let first_item = &expr[expr.root_index];
        match *first_item {
            MathItem::List(ref list) => {
                list.iter()
                    .cloned()
                    .filter(|&index| if let MathItem::Operator(_) = expr[index] {
                                true
                            } else {
                                false
                            })
                    .next()
                    .expect("List contains no operator.")
            }
            MathItem::Operator(_) => 0.into(),
            ref other_item => panic!("Expected list or Operator. Found {:?}", other_item),
        }
    }

    #[test]
    fn test_operator() {
        let xml = "<mo>+</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match expr[operator] {
            MathItem::Operator(Operator { field: Field::Unicode(ref text), .. }) => {
                assert_eq!(text, "+")
            }
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_prefix_operator() {
        let xml = "<mo>-</mo><mi>x</mi>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match expr[operator] {
            MathItem::Operator(Operator { field: Field::Unicode(ref text), .. }) => {
                assert_eq!(text, "\u{2212}") // MINUS SIGN
            }
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_infix_operator() {
        let xml = "<mi>x</mi><mo>=</mo><mi>y</mi>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match expr[operator] {
            MathItem::Operator(Operator { field: Field::Unicode(ref text), .. }) => {
                assert_eq!(text, "=")
            }
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_postfix_operator() {
        let xml = "<mi>x</mi><mo>!</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match expr[operator] {
            MathItem::Operator(Operator { field: Field::Unicode(ref text), .. }) => {
                assert_eq!(text, "!")
            }
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }
}
