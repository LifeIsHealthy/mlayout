use crate::types::{
    Atom, GeneralizedFraction, Length, MathExpression, MathItem, Operator, OverUnder,
    StretchConstraints,
};

use super::operator_dict;
use super::{FromXmlAttribute, ParseContext};

bitflags! {
    pub struct Flags: u8 {
        const SYMMETRIC         = 0b00000001;
        const FENCE             = 0b00000010;
        const STRETCHY          = 0b00000100;
        const SEPARATOR         = 0b00001000;
        const ACCENT            = 0b00010000;
        const LARGEOP           = 0b00100000;
        const MOVABLE_LIMITS    = 0b01000000;
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
            _ => Err(FormParsingError {
                unknown_str: String::from_utf8_lossy(s).into_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Default)]
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
//
// After we have fully parsed a `mrow` of math elements we have to look at it again to find out
// which default attributes to apply to the operators. For every operator this depends on whether it is at the
// beginning/end or in the middle of a `mrow` (ignoring any whitespace elements).
pub fn process_operators(list: &mut Vec<MathExpression>, context: &mut ParseContext) {
    // filter out all whitespace elements
    let non_whitespace_list = list
        .iter_mut()
        .filter(|expr| {
            context
                .info_for_expr(&**expr)
                .map(|info| !info.is_space)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    let len = non_whitespace_list.len();
    for (i, mut expr) in non_whitespace_list.into_iter().enumerate() {
        if !context
            .info_for_expr(&*expr)
            .map(|info| info.is_operator())
            .unwrap_or(false)
        {
            // current element is not an operator, nothing to do
            continue;
        }
        if len > 1 {
            if i == 0 {
                set_default_form(&expr, Form::Prefix, context);
            } else if i == len - 1 {
                set_default_form(&expr, Form::Postfix, context);
            }
        }

        set_default_form(&expr, Form::Infix, context);
        guess_operator_attributes(&expr, context);
        make_operator(&mut expr, context);
    }
}

/// Guess the default attributes of a math operator.
///
/// This function will create a `MathExpression` representing an operator with the correct default
/// arguments according to the MathML spec.
///
/// # Arguments
/// - `expr`: The operator whose attributes to guess.
/// - `form`: Which default form to assume (this has to be decided given the surrounding elements).
/// - `context`: The context for the MathML parser.
pub(super) fn guess_if_operator_with_form(
    mut expr: MathExpression,
    form: Form,
    context: &mut ParseContext,
) -> MathExpression {
    set_default_form(&expr, form, context);
    guess_operator_attributes(&expr, context);
    make_operator(&mut expr, context);
    expr
}

fn set_default_form(expr: &MathExpression, form: Form, context: &mut ParseContext) {
    let info = context.info_for_expr_mut(expr);
    let operator_attrs = info.and_then(|info| info.operator_attrs.as_mut());
    let operator_attrs = match operator_attrs {
        Some(operator_attrs) => operator_attrs,
        None => return,
    };
    operator_attrs.form = operator_attrs.form.or(Some(form))
}

fn guess_operator_attributes(expr: &MathExpression, context: &mut ParseContext) {
    let info = context.info_for_expr_mut(expr);
    let operator_attrs = info.and_then(|info| info.operator_attrs.as_mut());
    let operator_attrs = match operator_attrs {
        Some(operator_attrs) => operator_attrs,
        None => return,
    };

    let form = operator_attrs.form.expect("operator has no form");
    let entry = operator_attrs
        .character
        .and_then(|chr| operator_dict::find_entry(chr, form))
        .unwrap_or_default();

    if operator_attrs.lspace.is_none() {
        operator_attrs.lspace = Some(Length::em(entry.lspace as f32 / 18.0f32));
    }
    if operator_attrs.rspace.is_none() {
        operator_attrs.rspace = Some(Length::em(entry.rspace as f32 / 18.0f32));
    }

    // apply user overrides
    operator_attrs.flags = (operator_attrs.user_overrides & operator_attrs.flags)
        | (!operator_attrs.user_overrides & entry.flags);
}

/// Recursively walk the MathExpression tree to find the core of an embellished operator.
fn find_core_operator<'a>(
    embellished_op: &'a mut MathExpression,
    context: &mut ParseContext,
) -> Option<&'a mut MathExpression> {
    if let &mut MathItem::Field(_) = embellished_op.item.as_mut() {
        return Some(embellished_op);
    }

    let core = match embellished_op.item.as_mut() {
        &mut MathItem::Atom(Atom {
            nucleus: Some(ref mut nucleus),
            ..
        }) => nucleus,
        &mut MathItem::OverUnder(OverUnder {
            nucleus: Some(ref mut nucleus),
            ..
        }) => nucleus,
        &mut MathItem::GeneralizedFraction(GeneralizedFraction {
            numerator: Some(ref mut numerator),
            ..
        }) => numerator,
        _ => return None,
    };
    find_core_operator(core, context)
}

fn set_movable_limits(embellished_op: &mut MathExpression, context: &mut ParseContext) {
    let mut core_expr = match *embellished_op.item {
        MathItem::Atom(Atom {
            nucleus: Some(ref mut nucleus),
            ..
        }) => nucleus,
        MathItem::OverUnder(ref mut ou) => {
            ou.is_limits = true;
            match ou.nucleus {
                Some(ref mut nucleus) => nucleus,
                None => return,
            }
        }
        MathItem::GeneralizedFraction(GeneralizedFraction {
            numerator: Some(ref mut numerator),
            ..
        }) => numerator,
        _ => return,
    };
    set_movable_limits(&mut core_expr, context)
}

/// Replace the `MathExpression` that represents the core operator by a `Operator`.
fn make_operator(expr: &mut MathExpression, context: &mut ParseContext) {
    let operator_attrs = {
        let info = context.info_for_expr(&*expr);
        match info.and_then(|info| info.operator_attrs.clone()) {
            Some(operator_attrs) => operator_attrs,
            None => return,
        }
    };

    let flags = operator_attrs.flags;

    if flags.contains(Flags::MOVABLE_LIMITS) {
        set_movable_limits(expr, context);
    }

    if let Some(ref mut core_expr) = find_core_operator(expr, context) {
        let stretch_constraints = if flags.contains(Flags::STRETCHY) {
            Some(StretchConstraints {
                symmetric: flags.contains(Flags::SYMMETRIC),
                ..Default::default()
            })
        } else {
            None
        };
        let field = match *core_expr.item {
            MathItem::Field(ref field) => field.clone(),
            _ => unreachable!(),
        };
        let new_elem = Operator {
            stretch_constraints: stretch_constraints,
            field: field,
            is_large_op: flags.contains(Flags::LARGEOP),
            leading_space: operator_attrs.lspace.expect("operator has no lspace"),
            trailing_space: operator_attrs.rspace.expect("operator has no rspace"),
            ..Default::default()
        };
        core_expr.item = Box::new(MathItem::Operator(new_elem));
    }
}

#[cfg(test)]
mod tests {
    use crate::mathmlparser::ParseContext;
    use stash::Stash;

    #[test]
    fn test_set_default_form() {
        let info = Stash::new();
        let mut context = ParseContext { mathml_info: info };
        let context = ParseContext {
            mathml_info: Stash::new(),
        };
    }
}
