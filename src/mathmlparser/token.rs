use std;
use std::borrow::Cow;
use std::io::BufRead;

use super::operator;
use super::error::ParsingError;
use super::{Result, ResultPos, MathmlElement, XmlReader, Event, MathmlInfo, ParseContext,
            FromXmlAttribute};
use crate::mathmlparser::AttributeParse;

use crate::types::{Field, MathItem, Index};
use super::escape::unescape;
use crate::unicode_math::{Family, convert_character_to_family};

#[derive(Debug)]
enum TextDirection {
    Ltr,
    Rtl,
}

impl FromXmlAttribute for TextDirection {
    type Err = ();
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err> {
        if bytes == b"rtl" {
            Ok(TextDirection::Rtl)
        } else {
            Ok(TextDirection::Ltr)
        }
    }
}

impl std::default::Default for TextDirection {
    fn default() -> TextDirection {
        TextDirection::Ltr
    }
}

impl FromXmlAttribute for Family {
    type Err = ();
    fn from_xml_attr(bytes: &[u8]) -> std::result::Result<Self, Self::Err> {
        match bytes {
            b"normal" => Ok(Family::Normal),
            b"bold" => Ok(Family::Bold),
            b"italic" => Ok(Family::Italics),
            b"bold-italic" => Ok(Family::BoldItalics),
            b"double-struck" => Ok(Family::DoubleStruck),
            b"bold-fraktur" => Ok(Family::BoldFraktur),
            b"script" => Ok(Family::Script),
            b"bold-script" => Ok(Family::BoldScript),
            b"fraktur" => Ok(Family::Fraktur),
            b"sans-serif" => Ok(Family::SansSerif),
            b"bold-sans-serif" => Ok(Family::SansSerifBold),
            b"sans-serif-italic" => Ok(Family::SansSerifItalics),
            b"sans-serif-bold-italic" => Ok(Family::SansSerifBoldItalics),
            b"monospace" => Ok(Family::Monospace),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default)]
struct TokenStyle {
    // If `math_variant` is None the family of the glyph depends on whether the element consists of
    // a single glyph or multiple glyphs. A single glyph is laid out in italic style. Multiple
    // glyphs would be layed out in normal style.
    math_variant: Option<Family>,
    // TODO: missing math_size
    direction: TextDirection,
}

#[allow(match_same_arms)]
fn parse_token_attribute<'a>(style: &mut TokenStyle,
                             element_identifier: &str,
                             new_attribute: &(&'a [u8], &'a [u8]))
                             -> bool {
    match *new_attribute {
        (b"mathvariant", variant) => style.math_variant = variant.parse_xml().ok(),
        (b"dir", dir) => style.direction = dir.parse_xml().unwrap(),
        _ => return false,
    }
    match (element_identifier, style.math_variant) {
        ("mi", None) => {}
        (_, None) => style.math_variant = Some(Family::Normal),
        _ => {}
    }
    true
}

fn adapt_to_family(text: &str, family: Option<Family>) -> Cow<str> {
    if family.is_none() {
        if text.chars().count() == 1 {
            let conv = convert_character_to_family(text.chars().next().unwrap(), Family::Italics);
            conv.to_string().into()
        } else {
            text.into()
        }
    } else {
        let family = family.unwrap();
        text.chars()
            .map(|chr| convert_character_to_family(chr, family))
            .collect::<String>()
            .into()
    }
}

fn replace_anomalous_characters(text: &str, elem: MathmlElement) -> String {
    text.chars()
        .map(|chr| match chr {
                 '-' if elem.identifier == "mo" => '\u{2212}', // Minus Sign
                 '-' => '\u{2010}', // Hyphen
                 '\u{0027}' => '\u{2023}', // Prime
                 chr => chr,

             })
        .collect()
}

// invoked after a token expression
// the cursor is moved behind the end element of the token expression
// the result (if ok) is guaranteed to not be empty
fn parse_token_contents<R: BufRead>(parser: &mut XmlReader<R>,
                                    elem: MathmlElement,
                                    style: TokenStyle)
                                    -> Result<Vec<Field>> {
    let mut fields: Vec<Field> = Vec::new();

    while let Some(event) = parser.next() {
        match event? {
            Event::Text(text) => {
                let text = std::str::from_utf8(text.content())?;
                let text = unescape(text)?;
                let string = adapt_to_family(&text, style.math_variant);
                let string = replace_anomalous_characters(&string, elem);
                fields.push(Field::Unicode(string));
            }
            Event::Start(elem) => {
                match elem.name() {
                    b"mglyph" | b"malignmark" => {
                        Err(ParsingError::from_string(parser,
                                                      format!("{:?} element is currently not \
                                                               implemented.",
                                                              elem.name())))?
                    }
                    _ => Err(ParsingError::from_string(parser, "Unexpected new element."))?,
                }
            }
            Event::End(ref end_elem) => {
                if elem.identifier.as_bytes() == end_elem.name() {
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(fields)
}

fn try_extract_char(field: &Field) -> Option<char> {
    if let Field::Unicode(ref string) = *field {
        let mut iterator = string.chars();
        if let Some(first_character) = iterator.next() {
            if iterator.next().is_none() {
                Some(first_character)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_operator_attribute(op_attrs: Option<&mut operator::Attributes>,
                            new_attr: &(&[u8], &[u8]))
                            -> bool {
    let mut op_attrs = match op_attrs {
        Some(op_attrs) => op_attrs,
        None => return false,
    };
    match *new_attr {
        (b"form", form_str) => op_attrs.form = form_str.parse_xml().ok(),
        (b"lspace", lspace) => {
            op_attrs.lspace = lspace.parse_xml().ok();
        }
        (b"rspace", rspace) => {
            op_attrs.rspace = rspace.parse_xml().ok();
        }
        (b"fence", is_fence) => {
            if let Ok(is_fence) = is_fence.parse_xml() {
                op_attrs.set_user_override(operator::FENCE, is_fence);
            }
        }
        (b"symmetric", is_symmetric) => {
            if let Ok(is_symmetric) = is_symmetric.parse_xml() {
                op_attrs.set_user_override(operator::SYMMETRIC, is_symmetric);
            }
        }
        (b"stretchy", is_stretchy) => {
            if let Ok(is_stretchy) = is_stretchy.parse_xml() {
                op_attrs.set_user_override(operator::STRETCHY, is_stretchy);
            }
        }
        (b"largeop", is_largeop) => {
            if let Ok(is_largeop) = is_largeop.parse_xml() {
                op_attrs.set_user_override(operator::LARGEOP, is_largeop);
            }
        }
        (b"movablelimits", has_movable_limits) => {
            if let Ok(has_movable_limits) = has_movable_limits.parse_xml() {
                op_attrs.set_user_override(operator::MOVABLE_LIMITS, has_movable_limits);
            }
        }
        (b"accent", is_accent) => {
            if let Ok(is_accent) = is_accent.parse_xml() {
                op_attrs.set_user_override(operator::ACCENT, is_accent);
            }
        }
        _ => return false,
    }
    true
}

pub fn parse<'a, R: BufRead, A>(parser: &mut XmlReader<R>,
                                elem: MathmlElement,
                                attributes: A,
                                context: &mut ParseContext)
                                -> Result<Index>
    where A: Iterator<Item = ResultPos<(&'a [u8], &'a [u8])>>
{
    let mut token_style = TokenStyle::default();
    let mut op_attrs = if elem.identifier == "mo" {
        Some(operator::Attributes::default())
    } else {
        None
    };
    attributes.filter_map(|attr| attr.ok())
        .filter(|attr| !parse_token_attribute(&mut token_style, elem.identifier, &attr))
        .filter(|attr| !parse_operator_attribute(op_attrs.as_mut(), &attr))
        .fold((), |_, _| {});
    let mut fields = parse_token_contents(parser, elem, token_style)?;
    let item = if fields.len() == 1 {
        let field = fields.remove(0);
        if let Some(ref mut op_attrs) = op_attrs {
            op_attrs.character = try_extract_char(&field);
        }
        MathItem::Field(field)
    } else {
        let list =
            fields.into_iter().map(|field| context.expr.add_item(MathItem::Field(field))).collect();
        MathItem::List(list)
    };

    let index = context.expr.add_item(item);
    context.mathml_info.insert(index.into(),
                               MathmlInfo {
                                   operator_attrs: op_attrs,
                                   ..Default::default()
                               });
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{match_math_element, Event, VecMap, MathExpression};

    fn test_operator_flag_parse(attr_name: &str, flag: operator::Flags) {
        let xml = format!("<mo {}=\"true\">a</mo>", attr_name);
        let mut parser = XmlReader::from(&xml as &str).trim_text(true);

        let elem = match parser.next().unwrap().unwrap() {
            Event::Start(elem) => elem,
            _ => panic!("Expected mo element"),
        };
        let mathml_elem = match_math_element(elem.name()).unwrap();
        let attributes = elem.attributes();

        let expr = MathExpression::new();
        let info = VecMap::new();
        let mut context = ParseContext {
            expr: expr,
            mathml_info: info,
        };
        let index = parse(&mut parser, mathml_elem, attributes, &mut context).unwrap();

        let operator_attrs = context.mathml_info
            .get(index.into())
            .unwrap()
            .operator_attrs
            .unwrap();
        assert!(operator_attrs.flags.contains(flag));
    }

    #[test]
    fn test_parse_operator_attributes() {
        test_operator_flag_parse("symmetric", operator::SYMMETRIC);
        test_operator_flag_parse("fence", operator::FENCE);
        test_operator_flag_parse("largeop", operator::LARGEOP);
        test_operator_flag_parse("stretchy", operator::STRETCHY);
    }
}
