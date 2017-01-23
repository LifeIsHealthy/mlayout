use std::fmt::Debug;

mod layout;
pub mod shaper;
pub mod math_box;
mod multiscripts;
pub mod unicode_math;
mod lazy_vec;

use types::*;
pub use self::layout::{MathBoxLayout, LayoutOptions};
use self::shaper::MathShaper;
use self::math_box::MathBox;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn layout<'a, T: 'a + Debug, S: MathShaper>(expression: MathExpression<T>,
                                                shaper: &'a S)
                                                -> MathBox<'a, T> {
    let options = LayoutOptions {
        shaper: shaper,
        style: LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
        },
        stretch_size: None,
    };

    expression.layout(options)
}
