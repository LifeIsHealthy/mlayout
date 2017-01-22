extern crate freetype;

use std::fmt::Debug;
use std::iter::*;


macro_rules! ot_tag {
    ($t1:expr, $t2:expr, $t3:expr, $t4:expr) => (
        (($t1 as u32) << 24) | (($t2 as u32) << 16) | (($t3 as u32) << 8) | ($t4 as u32)
    );
}

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
pub fn layout<T: Debug, S: MathShaper>(expression: MathExpression<T>,
                 shaper: &S,
                 ft_lib: &freetype::Library)
                 -> MathBox<T> {
    let options = LayoutOptions {
        shaper: shaper,
        style: LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
        },
        stretch_size: None,
        ft_library: ft_lib,
    };

    let boxes = expression.layout(options);
    boxes.collect()
}
