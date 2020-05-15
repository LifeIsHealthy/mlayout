#![allow(missing_docs)]
#![allow(unknown_lints)]

#[macro_use]
extern crate bitflags;
extern crate stash;
extern crate generational_arena;

mod types;
mod typesetting;
// mod layout_v2;

#[cfg(feature = "mathml_parser")]
extern crate quick_xml;
#[cfg(feature = "mathml_parser")]
pub mod mathmlparser;

pub use crate::typesetting::{math_box, unicode_math, shaper, layout};
pub use crate::types::*;
