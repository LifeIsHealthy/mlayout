use std::str::FromStr;
use std::ops::Not;

use types::{Length, MathItem, StretchConstraints, Operator};

use super::{MExpression, MathmlInfo};
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

impl FromStr for Form {
    type Err = FormParsingError;
    fn from_str(s: &str) -> Result<Form, FormParsingError> {
        match s {
            "prefix" => Ok(Form::Prefix),
            "infix" => Ok(Form::Infix),
            "postfix" => Ok(Form::Postfix),
            _ => Err(FormParsingError { unknown_str: s.to_string() }),
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
pub fn process_operators(list: &mut Vec<MExpression>) {
    let mut first_non_ws_index = -1;
    let mut last_non_ws_index = 0;
    let operator_indices = list.iter_mut()
        .enumerate()
        .filter(|&(_, ref elem)| elem.user_info.is_space.not())
        .inspect(|&(index, _)| {
            if first_non_ws_index == -1 {
                first_non_ws_index = index as isize;
            }
            last_non_ws_index = index;
        })
        .filter(|&(_, ref elem)| elem.user_info.is_operator())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    for index in &operator_indices {
        let elem = &mut list[*index];
        if first_non_ws_index as usize != last_non_ws_index {
            if *index == first_non_ws_index as usize {
                set_default_form(elem, Form::Prefix);
            } else if *index == last_non_ws_index {
                set_default_form(elem, Form::Postfix);
            }
        }
        set_default_form(elem, Form::Infix);
        guess_operator_attributes(elem);
        make_operator(elem);
    }
}

pub fn guess_if_operator_with_form(mut elem: MExpression, form: Form) -> MExpression {
    set_default_form(&mut elem, form);
    guess_operator_attributes(&mut elem);
    make_operator(&mut elem);
    elem
}

fn set_default_form(elem: &mut MExpression, form: Form) {
    elem.user_info
        .operator_attrs
        .as_mut()
        .map(|op_attrs| op_attrs.form = op_attrs.form.or(Some(form)));
}

fn guess_operator_attributes(elem: &mut MExpression) {
    if elem.user_info.operator_attrs.is_none() {
        return
    }
    let operator_attrs = elem.user_info.operator_attrs.as_mut().unwrap();

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

fn find_core_operator(embellished_op: &mut MExpression) -> Option<&mut MathItem<MathmlInfo>> {
    match embellished_op.content {
        MathItem::Field(_) => Some(&mut embellished_op.content),
        MathItem::Atom(ref mut atom) => find_core_operator(&mut atom.nucleus),
        MathItem::OverUnder(ref mut ou) => find_core_operator(&mut ou.nucleus),
        MathItem::GeneralizedFraction(ref mut frac) => find_core_operator(&mut frac.numerator),
        _ => None,
    }
}

fn set_movable_limits(embellished_op: &mut MExpression) {
    match embellished_op.content {
        MathItem::Atom(ref mut atom) => set_movable_limits(&mut atom.nucleus),
        MathItem::OverUnder(ref mut ou) => {
            ou.is_limits = true;
            set_movable_limits(&mut ou.nucleus)
        },
        MathItem::GeneralizedFraction(ref mut frac) => set_movable_limits(&mut frac.numerator),
        _ => {},
    }
}

fn make_operator(elem: &mut MExpression) {
    if elem.user_info.operator_attrs.is_none() {
        return
    }
    let operator_attrs = elem.user_info.operator_attrs.unwrap();
    let flags = operator_attrs.flags;

    if flags.contains(MOVABLE_LIMITS) {
        set_movable_limits(elem);
    }

    if let Some(item) = find_core_operator(elem) {
        let stretch_constraints = if flags.contains(STRETCHY) {
            Some(StretchConstraints { symmetric: flags.contains(SYMMETRIC), ..Default::default() })
        } else {
            None
        };
        let field = match *item {
            MathItem::Field(ref field) => field.clone(),
            _ => unreachable!()
        };
        let new_elem = Operator {
            stretch_constraints: stretch_constraints,
            field: field,
            is_large_op: flags.contains(LARGEOP),
            leading_space: operator_attrs.lspace.expect("operator has no lspace"),
            trailing_space: operator_attrs.rspace.expect("operator has no rspace"),
            ..Default::default()
        };
        ::std::mem::replace(item, MathItem::Operator(new_elem));
    }
}
