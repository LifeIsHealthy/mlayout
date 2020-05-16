mod escape;
mod operator;
mod operator_dict;
mod token;

mod error;
#[cfg(feature = "mathml_parser")]
mod xml_reader;
#[cfg(feature = "mathml_parser")]
pub use xml_reader::parse;

use std;

use crate::{
    types::{
        Atom, GeneralizedFraction, Length, LengthUnit, MathExpression, MathItem, OverUnder, Root,
    },
    Field,
};

use stash::Stash;

use self::operator::{guess_if_operator_with_form, Form};

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
    fn from_xml_attr(attr: &str) -> std::result::Result<Self, Self::Err>;
}

pub trait AttributeParse {
    fn parse_xml<T: FromXmlAttribute>(&self) -> std::result::Result<T, T::Err>;
}

impl AttributeParse for str {
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

pub fn match_math_element(identifier: &[u8]) -> Option<MathmlElement> {
    MATHML_ELEMENTS
        .iter()
        .find(|elem| elem.identifier.as_bytes() == identifier)
        .cloned()
}

#[derive(Clone, Debug, Default)]
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

pub enum Child {
    Field(Field),
    Expression(MathExpression),
}

pub fn build_element<'a>(
    elem: MathmlElement,
    attributes: impl Iterator<Item = (&'a str, &'a str)>,
    children: impl Iterator<Item = Child>,
    context: &mut ParseContext,
) -> MathExpression {
    match elem.elem_type {
        ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(_),
        } => {
            let expressions = children.filter_map(|child| match child {
                Child::Expression(expr) => Some(expr),
                _ => None,
            });
            parse_fixed_schema(expressions, elem, attributes, context)
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        }
        | ElementType::MathmlRoot => {
            let expressions = children.filter_map(|child| match child {
                Child::Expression(expr) => Some(expr),
                _ => None,
            });
            let mut list = expressions.collect();
            operator::process_operators(&mut list, context);
            parse_list_schema(list, elem)
        }
        ElementType::TokenElement => {
            let fields = children.filter_map(|child| match child {
                Child::Field(field) => Some(field),
                _ => None,
            });
            token::build_token(fields, elem, attributes, context).unwrap()
        }
        _ => todo!(),
    }
}

fn parse_list_schema<'a>(mut content: Vec<MathExpression>, elem: MathmlElement) -> MathExpression {
    // a mrow with a single element is strictly equivalent to the element
    let content = if content.len() == 1 {
        content.remove(0)
    } else {
        MathExpression::new(MathItem::List(content), ())
    };
    if elem.elem_type == ElementType::MathmlRoot {
        return content;
    }
    match elem.identifier {
        "mrow" | "math" => content,
        "msqrt" => {
            let item = Root {
                radicand: Some(content),
                ..Default::default()
            };
            MathExpression::new(MathItem::Root(item), ())
        }
        _ => content,
    }
}

fn construct_under_over<'a>(
    nucleus: Option<MathExpression>,
    under: Option<MathExpression>,
    over: Option<MathExpression>,
    attributes: impl Iterator<Item = (&'a str, &'a str)>,
    context: &mut ParseContext,
) -> MathItem {
    let over = over.map(|x| guess_if_operator_with_form(x, Form::Postfix, context));
    let under = under.map(|x| guess_if_operator_with_form(x, Form::Postfix, context));

    let mut over_is_accent = context
        .operator_attrs(over.as_ref())
        .map(|op_attrs| op_attrs.flags.contains(operator::Flags::ACCENT))
        .unwrap_or(false);

    let mut under_is_accent = context
        .operator_attrs(under.as_ref())
        .map(|op_attrs| op_attrs.flags.contains(operator::Flags::ACCENT))
        .unwrap_or(false);

    // now check the accent attributes of the mover/munder element.
    for attrib in attributes {
        let (ident, value) = attrib;
        if ident == "accent" {
            over_is_accent = value.parse_xml().unwrap_or(false);
        }
        if ident == "accentunder" {
            under_is_accent = value.parse_xml().unwrap_or(false);
        }
    }

    let item = OverUnder {
        nucleus,
        under,
        over,
        over_is_accent,
        under_is_accent,
        ..Default::default()
    };

    MathItem::OverUnder(item)
}

fn parse_fixed_schema<'a, A>(
    mut content: impl Iterator<Item = MathExpression>,
    elem: MathmlElement,
    attributes: A,
    context: &mut ParseContext,
) -> MathExpression
where
    A: Iterator<Item = (&'a str, &'a str)>,
{
    let mut next = || Some(content.next().unwrap());
    let result = match elem.identifier {
        "mfrac" => {
            let frac = GeneralizedFraction {
                numerator: next(),
                denominator: next(),
                thickness: None,
            };
            MathItem::GeneralizedFraction(frac)
        }
        "mroot" => {
            let root = Root {
                radicand: next(),
                degree: next(),
            };
            MathItem::Root(root)
        }
        "msub" => {
            let atom = Atom {
                nucleus: next(),
                bottom_right: Some(guess_if_operator_with_form(
                    next().unwrap(),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msup" => {
            let atom = Atom {
                nucleus: next(),
                top_right: Some(guess_if_operator_with_form(
                    next().unwrap(),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "msubsup" => {
            let atom = Atom {
                nucleus: next(),
                bottom_right: Some(guess_if_operator_with_form(
                    next().unwrap(),
                    Form::Postfix,
                    context,
                )),
                top_right: Some(guess_if_operator_with_form(
                    next().unwrap(),
                    Form::Postfix,
                    context,
                )),
                ..Default::default()
            };
            MathItem::Atom(atom)
        }
        "mover" => {
            let nuc = next();
            let over = next();
            construct_under_over(nuc, None, over, attributes, context)
        }
        "munder" => {
            let nuc = next();
            let under = next();
            construct_under_over(nuc, under, None, attributes, context)
        }
        "munderover" => {
            let nuc = next();
            let under = next();
            let over = next();
            construct_under_over(nuc, under, over, attributes, context)
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
    expr
}

impl FromXmlAttribute for Length {
    type Err = &'static str;
    fn from_xml_attr(attr: &str) -> std::result::Result<Self, Self::Err> {
        let string = attr.trim().to_ascii_lowercase();
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
    type Err = &'static str;
    fn from_xml_attr(bytes: &str) -> std::result::Result<Self, Self::Err> {
        match bytes {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("unrecognized boolean value"),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "mathml_parser")]
mod tests {
    use super::*;
    use crate::types::*;
    use xml_reader::parse;

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
