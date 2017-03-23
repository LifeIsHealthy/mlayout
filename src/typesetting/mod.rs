mod layout;
pub mod shaper;
pub mod math_box;
mod multiscripts;
pub mod unicode_math;
pub mod lazy_vec;
mod stretchy;

use types::*;
pub use self::layout::{MathLayout, LayoutOptions, layout_expression};
use self::shaper::MathShaper;
use self::math_box::MathBox;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn layout<'a, S: MathShaper>(expression: &'a MathExpression, shaper: &'a S) -> MathBox<'a> {
    let options = LayoutOptions {
        shaper: shaper,
        style: LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
            flat_accent: false,
        },
        stretch_size: None,
        as_accent: false,
    };

    layout::layout_expression(expression, options)
}
