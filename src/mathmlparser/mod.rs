mod error;
mod operator;
mod operator_dict;
mod token;
mod escape;

use std;
use std::io::BufRead;

use types::{Atom, GeneralizedFraction, Length, MathExpression, MathItem, OverUnder, Root};

pub use quick_xml::{Element, Event, XmlReader};
pub use quick_xml::error::ResultPos;
use stash::Stash;

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
    ArgumentList,          // single argument or inferred mrow
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
static MATHML_ELEMENTS: [MathmlElement; 16] = [
    MathmlElement {
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
        identifier: "mspace",
        elem_type: ElementType::TokenElement,
    },
    MathmlElement {
        identifier: "mrow",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        },
    },
    MathmlElement {
        identifier: "math",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        },
    },
    MathmlElement {
        identifier: "msub",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
    MathmlElement {
        identifier: "msup",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
    MathmlElement {
        identifier: "msubsup",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(3),
        },
    },
    MathmlElement {
        identifier: "mover",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
    MathmlElement {
        identifier: "munder",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
    MathmlElement {
        identifier: "munderover",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(3),
        },
    },
    MathmlElement {
        identifier: "mfrac",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
    MathmlElement {
        identifier: "msqrt",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        },
    },
    MathmlElement {
        identifier: "mroot",
        elem_type: ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(2),
        },
    },
];

fn match_math_element(identifier: &[u8]) -> Option<MathmlElement> {
    for elem in MATHML_ELEMENTS.iter() {
        if elem.identifier.as_bytes() == identifier {
            return Some(*elem);
        }
    }
    None
}

pub struct ParseContext {
    pub mathml_info: Stash<MathmlInfo>,
}

impl ParseContext {
    fn info_for_expr<'a, T: Into<Option<&'a MathExpression>>>(&self, expr: T) -> Option<&MathmlInfo> {
        if let Some(&index) = expr.into().and_then(|x| x.downcast_user_data_ref()) {
            self.mathml_info.get(index)
        } else {
            None
        }
    }

    fn info_for_expr_mut<'a, T: Into<Option<&'a MathExpression>>>(&mut self, expr: T) -> Option<&mut MathmlInfo> {
        if let Some(&index) = expr.into().and_then(|x| x.downcast_user_data_ref()) {
            self.mathml_info.get_mut(index)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone)]
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
    let info = Stash::new();
    let mut context = ParseContext { mathml_info: info };

    parse_element(&mut parser, root_elem, std::iter::empty(), &mut context)
}

fn parse_element<'a, R: BufRead, A>(
    parser: &mut XmlReader<R>,
    elem: MathmlElement,
    attributes: A,
    context: &mut ParseContext,
) -> Result<MathExpression>
where
    A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>,
{
    match elem.elem_type {
        ElementType::TokenElement => token::parse(parser, elem, attributes, context),
        ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        }
        | ElementType::MathmlRoot => {
            let mut list = parse_element_list(parser, elem, context)?;
            operator::process_operators(&mut list, context);
            parse_list_schema(list, elem, attributes, context)
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(_),
        } => {
            let arguments = parse_fixed_arguments(parser, elem, context);
            parse_fixed_schema(arguments?, elem, attributes, context)
        }
        _ => unimplemented!(),
    }
}

fn parse_sub_element<R: BufRead>(
    parser: &mut XmlReader<R>,
    elem: &Element,
    context: &mut ParseContext,
) -> Result<MathExpression> {
    let sub_elem = match_math_element(elem.name());
    match sub_elem {
        Some(sub_elem) => parse_element(parser, sub_elem, elem.attributes(), context),
        None => {
            let name = String::from_utf8_lossy(elem.name()).into_owned();
            let result: Result<_> = parser.read_to_end(elem.name()).map_err(|err| err.into());
            result.and(Err(ParsingError::of_type(
                parser,
                ErrorType::UnknownElement(name),
            )))
        }
    }
}

fn parse_element_list<R: BufRead>(
    parser: &mut XmlReader<R>,
    elem: MathmlElement,
    context: &mut ParseContext,
) -> Result<Vec<MathExpression>> {
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
                    return Err(ParsingError::of_type(
                        parser,
                        ErrorType::WrongEndElement(name),
                    ));
                }
                if end_elem.name() == elem.identifier.as_bytes() {
                    break;
                } else {
                    let name = std::str::from_utf8(end_elem.name())?.to_string();
                    return Err(ParsingError::of_type(
                        parser,
                        ErrorType::WrongEndElement(name),
                    ));
                }
            }
            Some(Err(error)) => Err(error)?,
            None => {
                if elem.elem_type == ElementType::MathmlRoot {
                    break;
                } else {
                    return Err(ParsingError::of_type(
                        parser,
                        ErrorType::UnexpectedEndOfInput,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(list)
}

fn parse_list_schema<'a, A>(
    mut content: Vec<MathExpression>,
    elem: MathmlElement,
    _attributes: A,
    _context: &mut ParseContext,
) -> Result<MathExpression>
where
    A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>,
{
    // a mrow with a single element is strictly equivalent to the element
    let content = if content.len() == 1 {
        content.remove(0)
    } else {
        MathExpression::new(MathItem::List(content), ())
    };
    if elem.elem_type == ElementType::MathmlRoot {
        return Ok(content);
    }
    match elem.identifier {
        "mrow" | "math" => Ok(content),
        "msqrt" => {
            let item = Root {
                radicand: Some(content),
                ..Default::default()
            };
            Ok(MathExpression::new(MathItem::Root(item), ()))
        }
        _ => unimplemented!(),
    }
}

fn parse_fixed_arguments<'a, R: BufRead>(
    parser: &mut XmlReader<R>,
    elem: MathmlElement,
    context: &mut ParseContext,
) -> Result<Vec<MathExpression>> {
    if let ElementType::LayoutSchema {
        args: ArgumentRequirements::RequiredArguments(num_args),
    } = elem.elem_type
    {
        let args = parse_element_list(parser, elem, context)?;
        if args.len() == num_args as usize {
            Ok(args)
        } else {
            Err(ParsingError::from_string(
                parser,
                format!(
                    "\"{:?}\" element requires {:?} arguments. \
                     Found {:?} arguments.",
                    elem.identifier,
                    num_args,
                    args.len()
                ),
            ))
        }
    } else {
        unreachable!();
    }
}

fn parse_fixed_schema<'a, A>(
    mut content: Vec<MathExpression>,
    elem: MathmlElement,
    attributes: A,
    context: &mut ParseContext,
) -> Result<MathExpression>
where
    A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>,
{
    let result = match elem.identifier {
        "mfrac" => {
            let frac = GeneralizedFraction {
                numerator: Some(content.remove(0)),
                denominator: Some(content.remove(0)),
                thickness: None,
            };
            MathItem::GeneralizedFraction(frac)
        }
        "mroot" => {
            let root = Root {
                radicand: Some(content.remove(0)),
                degree: Some(content.remove(0)),
            };
            MathItem::Root(root)
        }
        "msub" => {
            let atom = Atom {
                nucleus: Some(content.remove(0)),
                bottom_right: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msup" => {
            let atom = Atom {
                nucleus: Some(content.remove(0)),
                top_right: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msubsup" => {
            let atom = Atom {
                nucleus: Some(content.remove(0)),
                bottom_right: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                top_right: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
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
                nucleus: Some(content.remove(0)),
                over: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                over_is_accent: as_accent,
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        "munder" => {
            let item = OverUnder {
                nucleus: Some(content.remove(0)),
                under: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        "munderover" => {
            let item = OverUnder {
                nucleus: Some(content.remove(0)),
                under: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                over: Some(guess_if_operator_with_form(content.remove(0), Form::Postfix, context)),
                ..Default::default()
            };
            MathItem::OverUnder(item)
        }
        _ => unreachable!(),
    };
    let info = MathmlInfo {
        operator_attrs: match result {
            MathItem::Atom(ref atom) => context
                .info_for_expr(atom.nucleus.as_ref())
                .and_then(|info| info.operator_attrs.clone()),
            MathItem::OverUnder(ref ou) => context
                .info_for_expr(ou.nucleus.as_ref())
                .and_then(|info| info.operator_attrs.clone()),
            MathItem::GeneralizedFraction(ref frac) => context
                .info_for_expr(frac.numerator.as_ref())
                .and_then(|info| info.operator_attrs.clone()),
            _ => None,
        },
        ..Default::default()
    };
    let index = context.mathml_info.put(info);
    let expr = MathExpression::new(result, index);
    Ok(expr)
}

impl FromXmlAttribute for Length {
    type Err = ParsingError;
    fn from_xml_attr(_: &[u8]) -> std::result::Result<Self, Self::Err> {
        println!("Length Parsing not yet implemented...");
        Ok(Length::default())
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

    fn find_operator(expr: &MathExpression) -> &MathExpression {
        match *expr.item {
            MathItem::List(ref list) => list.iter()
                .filter(|&expr| {
                    if let MathItem::Operator(_) = *expr.item {
                        true
                    } else {
                        false
                    }
                })
                .next()
                .expect("List contains no operator."),
            MathItem::Operator(_) => expr,
            ref other_item => panic!("Expected list or Operator. Found {:?}", other_item),
        }
    }

    #[test]
    fn test_operator() {
        let xml = "<mo>+</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match *operator.item {
            MathItem::Operator(Operator {
                field: Field::Unicode(ref text),
                ..
            }) => assert_eq!(text, "+"),
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_prefix_operator() {
        let xml = "<mo>-</mo><mi>x</mi>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match *operator.item {
            MathItem::Operator(Operator {
                field: Field::Unicode(ref text),
                ..
            }) => {
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
        match *operator.item {
            MathItem::Operator(Operator {
                field: Field::Unicode(ref text),
                ..
            }) => assert_eq!(text, "="),
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }

    #[test]
    fn test_postfix_operator() {
        let xml = "<mi>x</mi><mo>!</mo>";
        let expr = parse(xml.as_bytes()).unwrap();
        let operator = find_operator(&expr);
        match *operator.item {
            MathItem::Operator(Operator {
                field: Field::Unicode(ref text),
                ..
            }) => assert_eq!(text, "!"),
            ref other_item => panic!("Expected MathItem::Operator. Found {:?}.", other_item),
        }
    }
}
