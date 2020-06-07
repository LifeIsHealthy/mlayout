#![allow(missing_docs)]
#![allow(unknown_lints)]

#[macro_use]
extern crate bitflags;

mod types;
mod typesetting;

#[cfg(feature = "mathml_parser")]
extern crate quick_xml;

pub mod mathmlparser;

pub use crate::typesetting::{math_box, unicode_math, shaper, layout, layout_with_style};
pub use crate::types::*;
