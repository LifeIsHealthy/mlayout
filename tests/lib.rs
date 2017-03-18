#![cfg(feature = "mathml_parser")]

extern crate math_render;
extern crate freetype;

use math_render::mathmlparser;
use math_render::math_box::{MathBoxContent, MathBoxMetrics};

mod util;
use util::TEST_FONT;

#[test]
fn mathml_test() {
    let bytes = include_bytes!("testfiles/simple.xml");
    mathmlparser::parse(&bytes[..]).expect("invalid parse");
}

#[test]
fn shaping_test() {
    TEST_FONT.with(|font| {
                       let bytes = include_bytes!("testfiles/schrödinger.xml");
                       let list = mathmlparser::parse(&bytes[..]).expect("invalid parse");
                       println!("{:?}", math_render::layout(&list, font));
                   })
}

fn assume_boxes<'a, 'b, T, G>(content: &MathBoxContent<T, G>)
                           -> &T {
    match *content {
        MathBoxContent::Boxes(ref list) => list,
        _ => panic!(),
    }
}

#[test]
fn no_scale_division_test() {
    TEST_FONT.with(|font| {
        let xml = "<mi>ab</mi>";
        let list = mathmlparser::parse(xml.as_bytes()).unwrap();
        println!("{:#?}", list);
        let result = math_render::layout(&list, font);
        println!("{:#?}", &result);
        let content = result.content();
        let boxes = assume_boxes(content).as_slice();
        // test that the second box has a greater x-value than the right edge of the first box
        // with a somewhat big error margin
        assert!(boxes[1].origin.x > (boxes[0].origin.x + (boxes[0].extents().width as f32 * 0.8) as i32));
    })
}

#[test]
fn fraction_centering_test() {
    TEST_FONT.with(|font| {
        let xml = "<mfrac><mn>1</mn><mn>2</mn></mfrac>";
        let list = mathmlparser::parse(xml.as_bytes()).unwrap();
        let result = math_render::layout(&list, font);
        println!("{:?}", result);
        let content = result.content();
        let boxes = assume_boxes(content).as_slice();

        // test that the second box has a greater x-value than the right edge of the first box
        // with a somewhat big error margin
        let fraction_bar = &boxes[1];
        let left_edge = fraction_bar.origin.x;
        let width = fraction_bar.extents().width;

        // test if the numerator is centered
        let num = &boxes[0];
        let left_space = num.origin.x - left_edge;
        let right_space = width - num.extents().width - left_space;
        println!("(left, right) = {:?}", (left_space, right_space));
        // allow rounding errors
        assert!((left_space - right_space).abs() <= 2);

        // test if the denominator is centered
        let denom = &boxes[2];
        let left_space = denom.origin.x - left_edge;
        let right_space = width - denom.extents().width - left_space;
        println!("(left, right) = {:?}", (left_space, right_space));
        // allow rounding errors
        assert!((left_space - right_space).abs() <= 2);
    })
}
