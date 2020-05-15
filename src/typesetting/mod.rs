pub mod math_box;
mod layout;
pub mod shaper;
mod multiscripts;
pub mod unicode_math;
mod stretchy;

use crate::types::*;
pub use self::layout::{layout_expression, LayoutOptions, MathLayout};
use self::shaper::MathShaper;
use self::math_box::MathBox;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn layout<'a, S>(expression: &'a MathExpression, shaper: &'a S) -> MathBox
where
    S: MathShaper,
{
    let options = LayoutOptions {
        shaper: shaper,
        style: LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
            flat_accent: false,
            stretch_constraints: None,
            as_accent: false,
        },
        stretch_size: None,
    };

    layout::layout_expression(expression, options)
}
