use std;
use std::borrow::Cow;
use std::io::BufRead;

use super::operator;
use super::quick_xml;
use super::error::ParsingError;
use super::{Result, ResultPos, MathmlElement, XmlReader, Event, MExpression, MathmlInfo,
            parse_length, parse_bool};

use types::{Field, MathItem, MathExpression};
use super::escape::unescape;
use unicode_math::{Family, convert_character_to_family};

#[derive(Debug)]
enum TextDirection {
    Ltr,
    Rtl,
}

impl std::default::Default for TextDirection {
    fn default() -> TextDirection {
        TextDirection::Ltr
    }
}

#[derive(Debug, Default)]
struct TokenStyle {
    // If `math_variant` is None the family of the glyph depends on whether the element consists
    // of a single glyph or multiple glyphs. A single glyph is layed out in italic style.
    // Multiple glyphs would be layed out in normal style.
    math_variant: Option<Family>,
    // TODO: missing math_size
    direction: TextDirection,
}

fn variant_parse(bytes: &str) -> Option<Family> {
    match bytes {
        "normal" => Some(Family::Normal),
        "bold" => Some(Family::Bold),
        "italic" => Some(Family::Italics),
        "bold-italic" => Some(Family::BoldItalics),
        "double-struck" => Some(Family::DoubleStruck),
        "bold-fraktur" => Some(Family::BoldFraktur),
        "script" => Some(Family::Script),
        "bold-script" => Some(Family::BoldScript),
        "fraktur" => Some(Family::Fraktur),
        "sans-serif" => Some(Family::SansSerif),
        "bold-sans-serif" => Some(Family::SansSerifBold),
        "sans-serif-italic" => Some(Family::SansSerifItalics),
        "sans-serif-bold-italic" => Some(Family::SansSerifBoldItalics),
        "monospace" => Some(Family::Monospace),
        _ => None,
    }
}

#[allow(match_same_arms)]
fn parse_token_attribute<'a>(mut style: TokenStyle,
                             element_identifier: &str,
                             new_attribute: &(&'a [u8], &'a str))
                             -> TokenStyle {
    match *new_attribute {
        (b"mathvariant", variant) => style.math_variant = variant_parse(variant),
        (b"dir", dir) => {
            if dir == "rtl" {
                style.direction = TextDirection::Rtl
            } else {
                style.direction = TextDirection::Ltr
            }
        }
        _ => {}
    }
    match (element_identifier, style.math_variant) {
        ("mi", None) => {}
        (_, None) => style.math_variant = Some(Family::Normal),
        _ => {}
    }
    style
}

fn adapt_to_family(text: String, family: Option<Family>) -> String {
    if family.is_none() {
        if text.len() == 1 {
            let conv = convert_character_to_family(text.chars().next().unwrap(), Family::Italics);
            conv.to_string()
        } else {
            text
        }
    } else {
        let family = family.unwrap();
        text.chars().map(|chr| convert_character_to_family(chr, family)).collect()
    }
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
        let result = match event? {
            Event::Text(text) => {
                let text = std::str::from_utf8(text.content())?;
                let text = unescape(text)?;
                let string = adapt_to_family(text.into_owned(), style.math_variant);
                Ok(Field::Unicode(string))
            }
            Event::Start(elem) => {
                match elem.name() {
                    b"mglyph" | b"malignmark" => unimplemented!(),
                    _ => Err(ParsingError::from_string(parser, "Unexpected new element.")),
                }
            }
            Event::End(ref end_elem) => {
                if elem.identifier.as_bytes() == end_elem.name() {
                    break;
                }
                continue;
            }
            _ => Err(ParsingError::from("Unknown error.")),
        };
        fields.push(result?);
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

fn parse_operator_attribute(op_attrs: Option<operator::Attributes>,
                                new_attr: &(&[u8], &str))
                                -> Option<operator::Attributes> {
    let mut op_attrs = if op_attrs.is_none() {
        return None;
    } else {
        op_attrs.unwrap()
    };
    match *new_attr {
        (b"form", form_str) => op_attrs.form = form_str.parse().ok(),
        (b"lspace", lspace) => {
            op_attrs.lspace = parse_length(lspace).ok();
        }
        (b"rspace", rspace) => {
            op_attrs.rspace = parse_length(rspace).ok();
        }
        (b"fence", is_fence) => {
            if let Ok(is_fence) = parse_bool(is_fence) {
                op_attrs.set_user_override(operator::FENCE, is_fence);
            }
        }
        (b"symmetric", is_symmetric) => {
            if let Ok(is_symmetric) = parse_bool(is_symmetric) {
                op_attrs.set_user_override(operator::SYMMETRIC, is_symmetric);
            }
        }
        (b"stretchy", is_stretchy) => {
            if let Ok(is_stretchy) = parse_bool(is_stretchy) {
                op_attrs.set_user_override(operator::STRETCHY, is_stretchy);
            }
        }
        (b"largeop", is_largeop) => {
            if let Ok(is_largeop) = parse_bool(is_largeop) {
                op_attrs.set_user_override(operator::LARGEOP, is_largeop);
            }
        }
        (b"movablelimits", has_movable_limits) => {
            if let Ok(has_movable_limits) = parse_bool(has_movable_limits) {
                op_attrs.set_user_override(operator::MOVABLE_LIMITS, has_movable_limits);
            }
        }
        (b"accent", is_accent) => {
            if let Ok(is_accent) = parse_bool(is_accent) {
                op_attrs.set_user_override(operator::ACCENT, is_accent);
            }
        }
        _ => {}
    }
    Some(op_attrs)
}

pub fn parse<'a, R: BufRead, A>(parser: &mut XmlReader<R>,
                                elem: MathmlElement,
                                attributes: A)
                                -> Result<MExpression>
    where A: Iterator<Item = ResultPos<(&'a [u8], Cow<'a, [u8]>)>>
{
    let token_style = TokenStyle::default();
    let op_attrs: Option<operator::Attributes> = if elem.identifier == "mo" {
        Some(Default::default())
    } else {
        None
    };
    let (token_style, mut op_attrs) = attributes.filter_map(|attr| attr.ok())
        .filter(|attr| std::str::from_utf8(&attr.1).is_ok())
        .fold((token_style, op_attrs), |(ts, oa), attr| {
            let attr = (attr.0, unsafe { std::str::from_utf8_unchecked(&attr.1) });
            (parse_token_attribute(ts, elem.identifier, &attr),
             parse_operator_attribute(oa, &attr))
        });
    let mut fields = parse_token_contents(parser, elem, token_style)?;
    let item = if fields.len() == 1 {
        let field = fields.remove(0);
        MathItem::Field(field)
    } else {
        let list = fields.into_iter()
            .map(|field| {
                MathExpression {
                    content: MathItem::Field(field),
                    user_info: Default::default(),
                }
            });
        MathItem::List(list.collect())
    };
    if let Some(ref mut op_attrs) = op_attrs {
        op_attrs.character = if let MathItem::Field(ref field) = item {
            try_extract_char(field)
        } else {
            None
        };
    }
    Ok(MathExpression {
        content: item,
        user_info: MathmlInfo { operator_attrs: op_attrs },
    })
}
