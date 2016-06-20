#![feature(specialization)]

extern crate math_render;

use math_render::mathmlparser;

#[test]
fn mathml_test() {
    let bytes = include_bytes!("testfiles/simple.xml");
    let list = mathmlparser::parse_file(&bytes[..]).expect("invalid parse");
    println!("{:?}", list);
}
