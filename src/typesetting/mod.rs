mod layout;
pub mod math_box;
mod multiscripts;
pub mod shaper;
mod stretchy;
pub mod unicode_math;

pub use self::layout::{layout_expression, LayoutOptions, MathLayout};
use self::math_box::MathBox;
use self::shaper::MathShaper;
use crate::types::*;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn layout<'a>(expression: &'a MathExpression, shaper: &'a impl MathShaper) -> MathBox {
    layout_with_style(expression, shaper, |old, _| old)
}

pub fn layout_with_style<'a>(
    expression: &'a MathExpression,
    shaper: &'a impl MathShaper,
    style: impl Fn(LayoutStyle, u64) -> LayoutStyle,
) -> MathBox {
    let user_data = expression.get_user_data();

    let default_style = LayoutStyle {
        math_style: MathStyle::Display,
        script_level: 0,
        is_cramped: false,
        flat_accent: false,
        stretch_constraints: None,
        as_accent: false,
    };

    let new_style = style(default_style, user_data);

    let options = LayoutOptions {
        shaper: shaper,
        style_provider: &style,
        style: new_style,
        stretch_size: None,
        user_data: expression.get_user_data(),
    };

    layout::layout_expression(expression, options)
}
