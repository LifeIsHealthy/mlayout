#![allow(missing_docs)]
#![allow(unknown_lints)]

#[macro_use]
extern crate bitflags;
extern crate stash;

mod types;
mod typesetting;

#[cfg(feature = "mathml_parser")]
pub mod mathmlparser;

pub use typesetting::*;
pub use types::*;
