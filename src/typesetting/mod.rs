use std::fmt::Debug;
use std::iter::*;

mod layout;
pub mod font;
pub mod math_box;
mod multiscripts;
pub mod unicode_math;

use types::*;
pub use self::layout::{MathBoxLayout, LayoutOptions};
use self::font::MathShaper;
use self::math_box::MathBox;

// Calculates the dimensions of the components and their relative positioning. However no space
// is distributed.
pub fn layout<T: Debug, S: MathShaper>(expression: MathExpression<T>, shaper: &S) -> MathBox<T> {
    let options = LayoutOptions {
        shaper: shaper,
        style: LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
        },
        stretch_size: None,
    };

    let boxes = expression.layout(options);
    boxes.collect()
}
