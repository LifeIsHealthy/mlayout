use std;
use std::borrow::Cow;

use super::operator;
use super::{
    error::ParsingError, FromXmlAttribute, MathmlElement, MathmlInfo,
    ParseContext,
};


use crate::types::{Field, Length, MathExpression, MathItem, MathSpace};
use crate::unicode_math::{convert_character_to_family, Family};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TextDirection {
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

#[derive(Debug, Copy, Clone, Default)]
pub struct TokenStyle {
    // If `math_variant` is None the family of the glyph depends on whether the element consists of
    // a single glyph or multiple glyphs. A single glyph is laid out in italic style. Multiple
    // glyphs would be layed out in normal style.
    pub math_variant: Option<Family>,
    // TODO: missing math_size
    pub direction: TextDirection,
}

pub trait StringExtMathml {
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

#[derive(Debug, Default, Copy, Clone)]
pub struct Attributes {
    pub operator_attributes: operator::Attributes,
    pub token_style: TokenStyle,
    pub horizontal_space: Option<Length>,
}

pub fn build_token<'a>(
    fields: impl Iterator<Item = (Field, u64)>,
    elem: MathmlElement,
    mut attributes: Attributes,
    context: &mut ParseContext,
    user_data: u64,
) -> Result<MathExpression, ParsingError> {
    if let Some(width) = attributes.horizontal_space {
        let item = MathExpression::new(
            MathItem::Space(MathSpace::horizontal_space(width)),
            user_data,
        );
        context.mathml_info.insert(
            user_data,
            MathmlInfo {
                operator_attrs: None,
                ..Default::default()
            },
        );
        return Ok(item);
    }

    let mut list = vec![];
    let mut first_field_char = None;
    for (field_num, field) in fields.enumerate() {
        let (field, field_user_data) = field;
        if field_num == 0 {
            first_field_char = try_extract_char(&field);
        }
        let expr = MathExpression::new(MathItem::Field(field), field_user_data);
        list.push(expr);
    }

    let expr = if list.len() == 1 {
        if elem.is("mo") {
            attributes.operator_attributes.character = first_field_char;
        }
        list.pop().unwrap()
    } else {
        MathExpression::new(MathItem::List(list), user_data)
    };

    context.mathml_info.insert(
        expr.get_user_data(),
        MathmlInfo {
            operator_attrs: if elem.is("mo") {
                Some(attributes.operator_attributes)
            } else {
                None
            },
            ..Default::default()
        },
    );

    Ok(expr)
}

#[cfg(test)]
#[cfg(feature = "mathml_parser")]
mod tests {
    use super::*;
    use crate::mathmlparser::{match_math_element, xml_reader::parse_token_contents};

    use quick_xml::{Event, XmlReader};

    // fn test_operator_flag_parse(attr_name: &str, flag: operator::Flags) {
    //     let xml = format!("<mo {}=\"true\">a</mo>", attr_name);
    //     let mut parser = XmlReader::from(&xml as &str).trim_text(true);

    //     let elem = match parser.next().unwrap().unwrap() {
    //         Event::Start(elem) => elem,
    //         _ => panic!("Expected mo element"),
    //     };
    //     let mathml_elem = match_math_element(elem.name()).unwrap();
    //     let attributes = elem.attributes();
    //     let attrs = attributes.filter_map(|res| {
    //         res.ok().and_then(|(a, b)| {
    //             Some((std::str::from_utf8(a).ok()?, std::str::from_utf8(b).ok()?))
    //         })
    //     });

    //     let mut context = ParseContext::default();
    //     let fields = parse_token_contents(&mut parser, mathml_elem).unwrap();
    //     let expr = build_token(fields, mathml_elem, attrs, &mut context, 0).unwrap();

    //     let operator_attrs = context
    //         .info_for_expr(&expr)
    //         .unwrap()
    //         .clone()
    //         .operator_attrs
    //         .unwrap();
    //     assert!(operator_attrs.flags.contains(flag));
    // }

    // #[test]
    // fn test_parse_operator_attributes() {
    //     test_operator_flag_parse("symmetric", operator::Flags::SYMMETRIC);
    //     test_operator_flag_parse("fence", operator::Flags::FENCE);
    //     test_operator_flag_parse("largeop", operator::Flags::LARGEOP);
    //     test_operator_flag_parse("stretchy", operator::Flags::STRETCHY);
    // }
}
