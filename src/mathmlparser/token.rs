use std;
use std::borrow::Cow;

use super::operator;
use super::{
    error::ParsingError, escape::StringExtUnescape, FromXmlAttribute, MathmlElement, MathmlInfo,
    ParseContext,
};
use crate::mathmlparser::AttributeParse;

use crate::types::{Field, Length, MathExpression, MathItem, MathSpace};
use crate::unicode_math::{convert_character_to_family, Family};

#[derive(Debug)]
enum TextDirection {
    Ltr,
    Rtl,
}

impl FromXmlAttribute for TextDirection {
    type Err = ();
    fn from_xml_attr(bytes: &str) -> std::result::Result<Self, Self::Err> {
        if bytes == "rtl" {
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
    fn from_xml_attr(bytes: &str) -> std::result::Result<Self, Self::Err> {
        match bytes {
            "normal" => Ok(Family::Normal),
            "bold" => Ok(Family::Bold),
            "italic" => Ok(Family::Italics),
            "bold-italic" => Ok(Family::BoldItalics),
            "double-struck" => Ok(Family::DoubleStruck),
            "bold-fraktur" => Ok(Family::BoldFraktur),
            "script" => Ok(Family::Script),
            "bold-script" => Ok(Family::BoldScript),
            "fraktur" => Ok(Family::Fraktur),
            "sans-serif" => Ok(Family::SansSerif),
            "bold-sans-serif" => Ok(Family::SansSerifBold),
            "sans-serif-italic" => Ok(Family::SansSerifItalics),
            "sans-serif-bold-italic" => Ok(Family::SansSerifBoldItalics),
            "monospace" => Ok(Family::Monospace),
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
fn parse_token_attribute<'a>(
    style: &mut TokenStyle,
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

trait StringExtMathml {
    fn adapt_to_family(&self, family: Option<Family>) -> Cow<str>;
    fn replace_anomalous_characters(&self, elem: MathmlElement) -> String;
}

impl StringExtMathml for str {
    fn adapt_to_family(&self, family: Option<Family>) -> Cow<str> {
        if family.is_none() {
            if self.chars().count() == 1 {
                let conv =
                    convert_character_to_family(self.chars().next().unwrap(), Family::Italics);
                conv.to_string().into()
            } else {
                self.into()
            }
        } else {
            let family = family.unwrap();
            self.chars()
                .map(|chr| convert_character_to_family(chr, family))
                .collect::<String>()
                .into()
        }
    }

    fn replace_anomalous_characters(&self, elem: MathmlElement) -> String {
        self.chars()
            .map(|chr| match chr {
                '-' if elem.identifier == "mo" => '\u{2212}', // Minus Sign
                '-' => '\u{2010}',                            // Hyphen
                '\u{0027}' => '\u{2023}',                     // Prime
                chr => chr,
            })
            .collect()
    }
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

fn parse_operator_attribute(
    op_attrs: Option<&mut operator::Attributes>,
    new_attr: &(&str, &str),
) -> bool {
    let op_attrs = match op_attrs {
        Some(op_attrs) => op_attrs,
        None => return false,
    };
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

pub fn build_token<'a>(
    fields: impl Iterator<Item = Field>,
    elem: MathmlElement,
    attributes: impl Iterator<Item = (&'a str, &'a str)>,
    context: &mut ParseContext,
) -> Result<MathExpression, ParsingError> {
    let mut token_style = TokenStyle::default();
    let mut op_attrs = if elem.identifier == "mo" {
        Some(operator::Attributes::default())
    } else {
        None
    };
    let mut space = None;
    attributes
        .filter(|attr| !parse_token_attribute(&mut token_style, elem.identifier, &attr))
        .filter(|attr| !parse_operator_attribute(op_attrs.as_mut(), &attr))
        .filter(|attr| !parse_mspace_attribute(&mut space, elem.identifier, &attr))
        .fold((), |_, _| {});

    if let Some(width) = space {
        let item = MathExpression::new(MathItem::Space(MathSpace::horizontal_space(width)), ());
        return Ok(item);
    }

    let mut fields = fields.map(|field| match field {
        Field::Unicode(string) => {
            let string = string.unescape().map(|string| {
                string
                    .adapt_to_family(token_style.math_variant)
                    .replace_anomalous_characters(elem)
            })?;
            Ok(Field::Unicode(string))
        }
        Field::Glyph(glyph) => Ok(Field::Glyph(glyph)),
        Field::Empty => Ok(Field::Empty),
    });

    let mut item = if fields.size_hint().1 == Some(1) {
        let field = fields.next().unwrap()?;
        if let Some(ref mut op_attrs) = op_attrs {
            op_attrs.character = try_extract_char(&field);
        }
        MathExpression::new(MathItem::Field(field), ())
    } else {
        let list = fields
            .map(|field: Result<_, ParsingError>| {
                Ok(MathExpression::new(MathItem::Field(field?), ()))
            })
            .collect::<Result<Vec<_>, ParsingError>>()?;
        MathExpression::new(MathItem::List(list), ())
    };

    let index = context.mathml_info.put(MathmlInfo {
        operator_attrs: op_attrs,
        ..Default::default()
    });

    item.set_user_data(index);
    Ok(item)
}

#[cfg(test)]
#[cfg(feature = "mathml_parser")]
mod tests {
    use super::*;
    use crate::mathmlparser::{match_math_element, xml_reader::parse_token_contents};

    use quick_xml::{Event, XmlReader};
    use stash::Stash;

    fn test_operator_flag_parse(attr_name: &str, flag: operator::Flags) {
        let xml = format!("<mo {}=\"true\">a</mo>", attr_name);
        let mut parser = XmlReader::from(&xml as &str).trim_text(true);

        let elem = match parser.next().unwrap().unwrap() {
            Event::Start(elem) => elem,
            _ => panic!("Expected mo element"),
        };
        let mathml_elem = match_math_element(elem.name()).unwrap();
        let attributes = elem.attributes();
        let attrs = attributes.filter_map(|res| {
            res.ok().and_then(|(a, b)| {
                Some((std::str::from_utf8(a).ok()?, std::str::from_utf8(b).ok()?))
            })
        });

        let info = Stash::new();
        let mut context = ParseContext { mathml_info: info };
        let fields = parse_token_contents(&mut parser, mathml_elem).unwrap();
        let expr = build_token(fields, mathml_elem, attrs, &mut context).unwrap();

        let operator_attrs = context
            .info_for_expr(&expr)
            .unwrap()
            .clone()
            .operator_attrs
            .unwrap();
        assert!(operator_attrs.flags.contains(flag));
    }

    #[test]
    fn test_parse_operator_attributes() {
        test_operator_flag_parse("symmetric", operator::Flags::SYMMETRIC);
        test_operator_flag_parse("fence", operator::Flags::FENCE);
        test_operator_flag_parse("largeop", operator::Flags::LARGEOP);
        test_operator_flag_parse("stretchy", operator::Flags::STRETCHY);
    }
}
