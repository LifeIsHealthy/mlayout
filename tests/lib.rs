extern crate math_render;
extern crate freetype;

use math_render::mathmlparser;
use math_render::math_box::{MathBox, Content};

mod util;

#[test]
fn mathml_test() {
    let bytes = include_bytes!("testfiles/simple.xml");
    mathmlparser::parse(&bytes[..]).expect("invalid parse");
}

#[test]
fn shaping_test() {
    let bytes = include_bytes!("testfiles/schrödinger.xml");
    let list = mathmlparser::parse(&bytes[..]).expect("invalid parse");
    println!("{:?}", util::layout_list(list));
    // panic!();
}

#[test]
fn no_scale_division_test() {

    let xml = "<mi>ab</mi>";
    let list = mathmlparser::parse(xml.as_bytes()).unwrap();
    let result = util::layout_list(list);
    match result {
        // test that the second box has a greater x-value than the right edge of the first box
        // with a somewhat big error margin
        MathBox { content: Content::Boxes(boxes), .. } => {
            assert!(boxes[1].origin.x >
                    (boxes[0].origin.x + (boxes[0].ink_extents.width as f32 * 0.8) as i32))
        }
        _ => panic!(),
    }
}

#[test]
fn fraction_centering_test() {
    let xml = "<mfrac><mn>1</mn><mn>2</mn></mfrac>";
    let list = mathmlparser::parse(xml.as_bytes()).unwrap();
    let result = util::layout_list(list);

    println!("{:?}", result);

    match result {
        // test that the second box has a greater x-value than the right edge of the first box
        // with a somewhat big error margin
        MathBox { content: Content::Boxes(boxes), .. } => {
            let fraction_bar = &boxes[1];
            let left_edge = fraction_bar.origin.x;
            let width = fraction_bar.ink_extents.width;

            // test if the numerator is centered
            let top_bounds = boxes[0].get_ink_bounds();
            let left_space = top_bounds.origin.x - left_edge;
            let right_space = width - top_bounds.extents.width - left_space;
            println!("(left, right) = {:?}", (left_space, right_space));
            // allow rounding errors
            assert!((left_space - right_space).abs() <= 2);

            // test if the denominator is centered
            let bottom_bounds = boxes[2].get_ink_bounds();
            let left_space = bottom_bounds.origin.x - left_edge;
            let right_space = width - bottom_bounds.extents.width - left_space;
            println!("(left, right) = {:?}", (left_space, right_space));
            // allow rounding errors
            assert!((left_space - right_space).abs() <= 2);
        }
        _ => panic!(),
    }
}
