mod error;
mod escape;
mod operator;
mod operator_dict;
mod token;

use std;
use std::io::BufRead;

use crate::types::{
    Atom, GeneralizedFraction, Length, LengthUnit, MathExpression, MathItem, OverUnder, Root,
};

pub use quick_xml::events::Event;
pub use quick_xml::{Reader, Result as XmlResult};
use stash::Stash;

pub use self::error::*;
use self::operator::{guess_if_operator_with_form, Form};

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

// a static list of all mathml elements known to this parser
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
    MATHML_ELEMENTS
        .iter()
        .find(|elem| elem.identifier.as_bytes() == identifier)
        .cloned()
}

pub struct ParseContext {
    pub mathml_info: Stash<MathmlInfo>,
}

impl ParseContext {
    fn info_for_expr<'a, T: Into<Option<&'a MathExpression>>>(
        &self,
        expr: T,
    ) -> Option<&MathmlInfo> {
        if let Some(&index) = expr.into().and_then(|x| x.downcast_user_data_ref()) {
            self.mathml_info.get(index)
        } else {
            None
        }
    }

    fn info_for_expr_mut<'a, T: Into<Option<&'a MathExpression>>>(
        &mut self,
        expr: T,
    ) -> Option<&mut MathmlInfo> {
        if let Some(&index) = expr.into().and_then(|x| x.downcast_user_data_ref()) {
            self.mathml_info.get_mut(index)
        } else {
            None
        }
    }

    fn operator_attrs<'a, 'b: 'a, T: Into<Option<&'a MathExpression>>>(
        &'b self,
        expr: T,
    ) -> Option<&'b operator::Attributes> {
        self.info_for_expr(expr)
            .and_then(|info| info.operator_attrs.as_ref())
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
    let mut parser = Reader::from_reader(file).trim_text(true);
    let root_elem = MathmlElement {
        identifier: "ROOT_ELEMENT", // this identifier is arbitrary and should not be used elsewhere
        elem_type: ElementType::MathmlRoot,
    };
    let info = Stash::new();
    let mut context = ParseContext { mathml_info: info };

    parse_element(&mut parser, root_elem, std::iter::empty(), &mut context)
}

fn parse_element<'a, R: BufRead, A>(
    parser: &mut Reader<R>,
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
        _ => Ok(content),
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

fn construct_under_over<'a>(
    nucleus: Option<MathExpression>,
    under: Option<MathExpression>,
    over: Option<MathExpression>,
    attributes: impl Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>,
    context: &mut ParseContext,
) -> Result<MathItem> {
    let over = over.map(|x| guess_if_operator_with_form(x, Form::Postfix, context));
    let under = under.map(|x| guess_if_operator_with_form(x, Form::Postfix, context));

    let mut over_is_accent = context
        .operator_attrs(over.as_ref())
        .map(|op_attrs| op_attrs.flags.contains(operator::ACCENT))
        .unwrap_or(false);

    let mut under_is_accent = context
        .operator_attrs(under.as_ref())
        .map(|op_attrs| op_attrs.flags.contains(operator::ACCENT))
        .unwrap_or(false);

    // now check the accent attributes of the mover/munder element.
    for attrib in attributes {
        let (ident, value) = attrib?;
        if ident == b"accent" {
            over_is_accent = value.parse_xml().unwrap_or(false);
        }
        if ident == b"accentunder" {
            under_is_accent = value.parse_xml().unwrap_or(false);
        }
    }

    let item = OverUnder {
        nucleus: nucleus,
        under: under,
        over: over,
        over_is_accent,
        under_is_accent,
        ..Default::default()
    };

    Ok(MathItem::OverUnder(item))
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
                bottom_right: Some(guess_if_operator_with_form(
                    content.remove(0),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msup" => {
            let atom = Atom {
                nucleus: Some(content.remove(0)),
                top_right: Some(guess_if_operator_with_form(
                    content.remove(0),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msubsup" => {
            let atom = Atom {
                nucleus: Some(content.remove(0)),
                bottom_right: Some(guess_if_operator_with_form(
                    content.remove(0),
                    Form::Postfix,
                    context,
                )),
                top_right: Some(guess_if_operator_with_form(
                    content.remove(0),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "mover" => {
            let nuc = Some(content.remove(0));
            let over = Some(content.remove(0));
            construct_under_over(nuc, None, over, attributes, context)?
        }
        "munder" => {
            let nuc = Some(content.remove(0));
            let under = Some(content.remove(0));
            construct_under_over(nuc, under, None, attributes, context)?
        }
        "munderover" => {
            let nuc = Some(content.remove(0));
            let under = Some(content.remove(0));
            let over = Some(content.remove(0));
            construct_under_over(nuc, under, over, attributes, context)?
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
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err> {
        let string = std::str::from_utf8(bytes)?.trim().to_ascii_lowercase();
        let textual_space = match &string[..] {
            "veryverythinmathspace" => Some(Length::em(1.0 / 18.0)),
            "verythinmathspace" => Some(Length::em(2.0 / 18.0)),
            "thinmathspace" => Some(Length::em(3.0 / 18.0)),
            "mediummathspace" => Some(Length::em(4.0 / 18.0)),
            "thickmathspace" => Some(Length::em(5.0 / 18.0)),
            "verythickmathspace" => Some(Length::em(6.0 / 18.0)),
            "veryverythickmathspace" => Some(Length::em(7.0 / 18.0)),
            "negativeveryverythinmathspace" => Some(Length::em(-1.0 / 18.0)),
            "negativeverythinmathspace" => Some(Length::em(-2.0 / 18.0)),
            "negativethinmathspace" => Some(Length::em(-3.0 / 18.0)),
            "negativemediummathspace" => Some(Length::em(-4.0 / 18.0)),
            "negativethickmathspace" => Some(Length::em(-5.0 / 18.0)),
            "negativeverythickmathspace" => Some(Length::em(-6.0 / 18.0)),
            "negativeveryverythickmathspace" => Some(Length::em(-7.0 / 18.0)),
            _ => None,
        };

        if let Some(x) = textual_space {
            return Ok(x);
        }

        let first_non_digit = string.find(|chr| match chr {
            '0'..='9' | '.' | '+' | '-' => false,
            _ => true,
        });
        let first_non_digit = match first_non_digit {
            Some(x) => x,
            None => string.len(),
        };
        if let Ok(num) = string[0..first_non_digit].parse() {
            let unit = match string[first_non_digit..].trim() {
                "em" => LengthUnit::Em,
                "pt" => LengthUnit::Point,
                // fallback to points
                _ => LengthUnit::Point,
            };
            Ok(Length::new(num, unit))
        } else {
            Err("invalid number")?
        }
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
            MathItem::List(ref list) => list
                .iter()
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
