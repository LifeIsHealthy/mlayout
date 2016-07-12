#![feature(specialization)]

extern crate math_render;

use math_render::mathmlparser;

#[test]
fn mathml_test() {
    let bytes = include_bytes!("testfiles/simple.xml");
    let list = mathmlparser::parse(&bytes[..]).expect("invalid parse");
    println!("{:?}", list);
    let mut size = std::mem::size_of_val(&list);
    for item in list {
        size += std::mem::size_of_val(&item);
    }
    println!("size in bytes {:?} B", size);
}
