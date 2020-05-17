use super::error::{ErrorType, ParsingError, Result};
use super::{
    match_math_element, operator, parse_fixed_schema, parse_list_schema, token,
    ArgumentRequirements, ElementType, MathmlElement, ParseContext,
};
use crate::{Field, MathExpression};
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
            let fields = parse_token_contents(parser, elem)?;
            Ok(token::build_token(fields, elem, attrs, context, user_data)?)
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::ArgumentList,
        }
        | ElementType::MathmlRoot => {
            let mut list = parse_element_list(parser, elem, context)?;
            operator::process_operators(&mut list, context);
            Ok(parse_list_schema(list, elem))
        }
        ElementType::LayoutSchema {
            args: ArgumentRequirements::RequiredArguments(_),
        } => {
            let arguments = parse_fixed_arguments(parser, elem, context)?;
            Ok(parse_fixed_schema(
                arguments.into_iter(),
                elem,
                attrs,
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
    // style: TokenStyle,
) -> Result<impl ExactSizeIterator<Item = Field>> {
    let mut fields: Vec<Field> = Vec::new();

    while let Some(event) = parser.next() {
        match event? {
            Event::Text(text) => {
                let text = std::str::from_utf8(text.content())?;
                fields.push(Field::Unicode(text.to_owned()));
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
