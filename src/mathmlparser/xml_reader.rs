use super::error::{ErrorType, ParsingError, Result};
use super::{
    escape::StringExtUnescape, match_math_element, operator, parse_fixed_schema, parse_list_schema,
    token, ArgumentRequirements, AttributeParse, ElementType, MathmlElement, ParseContext,
    SchemaAttributes, StringExtMathml,
};

use crate::{unicode_math::Family, Field, Length, MathExpression};
pub use quick_xml::error::ResultPos;
pub use quick_xml::{Element, Event, XmlReader};
use std::io::BufRead;

pub fn parse<R: BufRead>(file: R) -> Result<MathExpression> {
    let mut parser = XmlReader::from_reader(file).trim_text(true);
    let root_elem = MathmlElement {
        identifier: "ROOT_ELEMENT", // this identifier is arbitrary and should not be used elsewhere
        elem_type: ElementType::MathmlRoot,
    };
    let mut context = ParseContext::default();

    parse_element(&mut parser, root_elem, std::iter::empty(), &mut context)
}

pub fn parse_element<'a, R: BufRead, A>(
    parser: &mut XmlReader<R>,
    elem: MathmlElement,
    attributes: A,
    context: &mut ParseContext,
) -> Result<MathExpression>
where
    A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>,
{
    let attrs = attributes.filter_map(|res| {
        res.ok()
            .and_then(|(a, b)| Some((std::str::from_utf8(a).ok()?, std::str::from_utf8(b).ok()?)))
    });
    let user_data = context.mathml_info.len() as u64;
    match elem.elem_type {
        ElementType::TokenElement => {
            let mut token_style = token::TokenStyle::default();
            let mut op_attrs = operator::Attributes::default();
            let mut space = None;
            attrs
                .filter(|attr| !parse_token_attribute(&mut token_style, elem.identifier, &attr))
                .filter(|attr| {
                    if elem.is("mo") {
                        !parse_operator_attribute(&mut op_attrs, &attr)
                    } else {
                        true
                    }
                })
                .filter(|attr| !parse_mspace_attribute(&mut space, elem.identifier, &attr))
                .fold((), |_, _| {});

            let fields = parse_token_contents(parser, elem, token_style)?;

            let attributes = token::Attributes {
                operator_attributes: op_attrs,
                token_style,
                horizontal_space: space,
            };

            Ok(token::build_token(
                fields, elem, attributes, context, user_data,
            )?)
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        }
        | ElementType::MathmlRoot => {
            let mut list = parse_element_list(parser, elem, context)?;
            operator::process_operators(&mut list, context);
            Ok(parse_list_schema(list, elem, user_data))
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(_),
        } => {
            let mut attributes = SchemaAttributes::default();
            for attr in attrs {
                parse_schema_attribute(&mut attributes, &attr);
            }

            let arguments = parse_fixed_arguments(parser, elem, context)?;
            Ok(parse_fixed_schema(
                arguments.into_iter(),
                elem,
                attributes,
                context,
                user_data,
            ))
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

// invoked after a token expression
// the cursor is moved behind the end element of the token expression
// the result (if ok) is guaranteed to not be empty
pub fn parse_token_contents<R: BufRead>(
    parser: &mut XmlReader<R>,
    elem: MathmlElement,
    token_style: token::TokenStyle,
) -> Result<impl ExactSizeIterator<Item = (Field, u64)>> {
    let mut fields: Vec<(Field, u64)> = Vec::new();

    while let Some(event) = parser.next() {
        match event? {
            Event::Text(text) => {
                let text = std::str::from_utf8(text.content())?;

                let text = text.unescape().map(|text| {
                    text.adapt_to_family(token_style.math_variant)
                        .replace_anomalous_characters(elem)
                })?;

                fields.push((Field::Unicode(text), 0));
            }
            Event::Start(elem) => match elem.name() {
                b"mglyph" | b"malignmark" => Err(ParsingError::from_string(
                    parser,
                    format!(
                        "{:?} element is currently not \
                         implemented.",
                        elem.name()
                    ),
                ))?,
                _ => Err(ParsingError::from_string(parser, "Unexpected new element."))?,
            },
            Event::End(ref end_elem) => {
                if elem.identifier.as_bytes() == end_elem.name() {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(fields.into_iter())
}

#[allow(match_same_arms)]
fn parse_token_attribute<'a>(
    style: &mut token::TokenStyle,
    element_identifier: &str,
    new_attribute: &(&'a str, &'a str),
) -> bool {
    match *new_attribute {
        ("mathvariant", variant) => style.math_variant = variant.parse_xml().ok(),
        ("dir", dir) => style.direction = dir.parse_xml().unwrap(),
        _ => return false,
    }
    match (element_identifier, style.math_variant) {
        ("mi", None) => {}
        (_, None) => style.math_variant = Some(Family::Normal),
        _ => {}
    }
    true
}

fn parse_operator_attribute(op_attrs: &mut operator::Attributes, new_attr: &(&str, &str)) -> bool {
    match *new_attr {
        ("form", form_str) => op_attrs.form = form_str.parse_xml().ok(),
        ("lspace", lspace) => {
            op_attrs.lspace = lspace.parse_xml().ok();
        }
        ("rspace", rspace) => {
            op_attrs.rspace = rspace.parse_xml().ok();
        }
        ("fence", is_fence) => {
            if let Ok(is_fence) = is_fence.parse_xml() {
                op_attrs.set_user_override(operator::Flags::FENCE, is_fence);
            }
        }
        ("symmetric", is_symmetric) => {
            if let Ok(is_symmetric) = is_symmetric.parse_xml() {
                op_attrs.set_user_override(operator::Flags::SYMMETRIC, is_symmetric);
            }
        }
        ("stretchy", is_stretchy) => {
            if let Ok(is_stretchy) = is_stretchy.parse_xml() {
                op_attrs.set_user_override(operator::Flags::STRETCHY, is_stretchy);
            }
        }
        ("largeop", is_largeop) => {
            if let Ok(is_largeop) = is_largeop.parse_xml() {
                op_attrs.set_user_override(operator::Flags::LARGEOP, is_largeop);
            }
        }
        ("movablelimits", has_movable_limits) => {
            if let Ok(has_movable_limits) = has_movable_limits.parse_xml() {
                op_attrs.set_user_override(operator::Flags::MOVABLE_LIMITS, has_movable_limits);
            }
        }
        ("accent", is_accent) => {
            if let Ok(is_accent) = is_accent.parse_xml() {
                op_attrs.set_user_override(operator::Flags::ACCENT, is_accent);
            }
        }
        _ => return false,
    }
    true
}

fn parse_mspace_attribute(
    horiz_space: &mut Option<Length>,
    identifier: &str,
    new_attr: &(&str, &str),
) -> bool {
    if identifier != "mspace" {
        return false;
    }
    match *new_attr {
        ("width", width) => {
            if let Ok(width) = width.parse_xml() {
                *horiz_space = Some(width);
            }
            true
        }
        _ => false,
    }
}

fn parse_schema_attribute(attributes: &mut SchemaAttributes, new_attr: &(&str, &str)) {
    match *new_attr {
        ("accent", is_accent) => attributes.accent = is_accent.parse().unwrap(),
        ("accentunder", is_accent) => attributes.accentunder = is_accent.parse().unwrap(),
        _ => {}
    }
}
