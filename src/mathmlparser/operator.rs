use types::{Length, MathItem, StretchConstraints, Operator, Index, Atom, OverUnder,
            GeneralizedFraction};

use super::{FromXmlAttribute, ParseContext};
use super::operator_dict;

bitflags! {
    pub flags Flags: u8 {
        const SYMMETRIC         = 0b00000001,
        const FENCE             = 0b00000010,
        const STRETCHY          = 0b00000100,
        const SEPARATOR         = 0b00001000,
        const ACCENT            = 0b00010000,
        const LARGEOP           = 0b00100000,
        const MOVABLE_LIMITS    = 0b01000000,
    }
}

impl Default for Flags {
    fn default() -> Flags {
        Flags::empty()
    }
}

pub struct FormParsingError {
    pub unknown_str: String,
}

#[derive(PartialEq, Eq, Copy, Clone, Debug, Ord, PartialOrd)]
pub enum Form {
    Prefix,
    Infix,
    Postfix,
}

impl Default for Form {
    fn default() -> Form {
        Form::Infix
    }
}

impl FromXmlAttribute for Form {
    type Err = FormParsingError;
    fn from_xml_attr(s: &[u8]) -> Result<Form, FormParsingError> {
        match s {
            b"prefix" => Ok(Form::Prefix),
            b"infix" => Ok(Form::Infix),
            b"postfix" => Ok(Form::Postfix),
            _ => Err(FormParsingError { unknown_str: String::from_utf8_lossy(s).into_owned() }),
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Attributes {
    pub character: Option<char>,
    pub form: Option<Form>,
    pub lspace: Option<Length>,
    pub rspace: Option<Length>,
    pub flags: Flags,
    pub user_overrides: Flags,
}

impl Attributes {
    pub fn set_user_override(&mut self, flag: Flags, is_true: bool) {
        self.user_overrides.insert(flag);
        if is_true {
            self.flags.insert(flag);
        } else {
            self.flags.remove(flag);
        }
    }
}

// (Embellished) operators are treated specially because their default attribute values depend
// on the surrounding elements.
pub fn process_operators(list: &mut Vec<Index>, context: &mut ParseContext) {
    // filter out all whitespace elements
    let non_whitespace_list = list.iter()
        .cloned()
        .filter(|&index| {
                    context.mathml_info
                        .get(index.into())
                        .map(|info| !info.is_space)
                        .unwrap_or(true)
                })
        .collect::<Vec<_>>();

    for &index in &non_whitespace_list {
        if !context.mathml_info
                .get(index.into())
                .map(|info| info.is_operator())
                .unwrap_or(false) {
            // current element is not an operator, nothing to do
            continue;
        }
        if non_whitespace_list.len() > 1 {
            if index == *non_whitespace_list.first().unwrap() {
                set_default_form(index, Form::Prefix, context);
            } else if index == *non_whitespace_list.last().unwrap() {
                set_default_form(index, Form::Postfix, context);
            }
        }

        set_default_form(index, Form::Infix, context);
        guess_operator_attributes(index, context);
        make_operator(index, context);
    }
}

pub fn guess_if_operator_with_form(index: Index, form: Form, context: &mut ParseContext) -> Index {
    set_default_form(index, form, context);
    guess_operator_attributes(index, context);
    make_operator(index, context);
    index
}

fn set_default_form(index: Index, form: Form, context: &mut ParseContext) {
    let info = context.mathml_info.get_mut(index.into());
    let mut operator_attrs = match info.and_then(|info| info.operator_attrs.as_mut()) {
        Some(operator_attrs) => operator_attrs,
        None => return,
    };
    operator_attrs.form = operator_attrs.form.or(Some(form))
}

fn guess_operator_attributes(index: Index, context: &mut ParseContext) {
    let info = context.mathml_info.get_mut(index.into());
    let mut operator_attrs = match info.and_then(|info| info.operator_attrs.as_mut()) {
        Some(operator_attrs) => operator_attrs,
        None => return,
    };

    let form = operator_attrs.form.expect("operator has no form");
    let entry = operator_attrs.character
        .and_then(|chr| operator_dict::find_entry(chr, form))
        .unwrap_or_default();

    if operator_attrs.lspace.is_none() {
        operator_attrs.lspace = Some(Length::em(entry.lspace as f32 / 18.0f32));
    }
    if operator_attrs.rspace.is_none() {
        operator_attrs.rspace = Some(Length::em(entry.rspace as f32 / 18.0f32));
    }

    operator_attrs.flags = (operator_attrs.user_overrides & operator_attrs.flags) |
                           (!operator_attrs.user_overrides & entry.flags);
}

fn find_core_operator(embellished_op: Index, context: &mut ParseContext) -> Option<Index> {
    let core_index = match context.expr[embellished_op] {
        MathItem::Field(_) => return Some(embellished_op),
        MathItem::Atom(Atom { nucleus: Some(nucleus), .. }) => nucleus,
        MathItem::OverUnder(OverUnder { nucleus: Some(nucleus), .. }) => nucleus,
        MathItem::GeneralizedFraction(GeneralizedFraction { numerator: Some(numerator), .. }) => numerator,
        _ => return None,
    };
    find_core_operator(core_index, context)
}

fn set_movable_limits(embellished_op: Index, context: &mut ParseContext) {
    let core_index = match context.expr[embellished_op] {
        MathItem::Atom(Atom { nucleus: Some(nucleus), .. }) => nucleus,
        MathItem::OverUnder(ref mut ou @ OverUnder { nucleus: Some(nucleus), .. }) => {
            ou.is_limits = true;
            nucleus
        }
        MathItem::GeneralizedFraction(GeneralizedFraction { numerator: Some(numerator), .. }) => numerator,
        _ => return,
    };
    set_movable_limits(core_index, context)
}

fn make_operator(index: Index, context: &mut ParseContext) {
    let operator_attrs = {
        let info = context.mathml_info.get(index.into());
        match info.and_then(|info| info.operator_attrs) {
            Some(operator_attrs) => operator_attrs,
            None => return,
        }
    };

    let flags = operator_attrs.flags;

    if flags.contains(MOVABLE_LIMITS) {
        set_movable_limits(index, context);
    }

    if let Some(core_index) = find_core_operator(index, context) {
        let stretch_constraints = if flags.contains(STRETCHY) {
            Some(StretchConstraints {
                     symmetric: flags.contains(SYMMETRIC),
                     ..Default::default()
                 })
        } else {
            None
        };
        let field = match context.expr[core_index] {
            MathItem::Field(ref field) => field.clone(),
            _ => unreachable!(),
        };
        let new_elem = Operator {
            stretch_constraints: stretch_constraints,
            field: field,
            is_large_op: flags.contains(LARGEOP),
            leading_space: operator_attrs.lspace.expect("operator has no lspace"),
            trailing_space: operator_attrs.rspace.expect("operator has no rspace"),
            ..Default::default()
        };
        context.expr[core_index] = MathItem::Operator(new_elem);
    }
}

#[cfg(test)]
mod tests {
    use mathmlparser::ParseContext;
    use types::MathExpression;
    use mathmlparser::VecMap;

    #[test]
    fn test_set_default_form() {
        let expr = MathExpression::new();
        let info = VecMap::new();
        let mut context = ParseContext {
            expr: expr,
            mathml_info: info,
        };
        let context = ParseContext {
            expr: MathExpression::new(),
            mathml_info: VecMap::new(),
        };

    }
}
