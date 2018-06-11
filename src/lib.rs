#![allow(missing_docs)]
#![allow(unknown_lints)]

#[macro_use]
extern crate bitflags;
extern crate stash;

mod types;
mod typesetting;

#[cfg(feature = "mathml_parser")]
extern crate quick_xml;
#[cfg(feature = "mathml_parser")]
pub mod mathmlparser;

pub use typesetting::{math_box, unicode_math, shaper, layout};
pub use types::*;
