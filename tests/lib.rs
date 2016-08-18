extern crate math_render;

use math_render::mathmlparser;

mod font_tests;

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
    //panic!();
}

#[test]
fn shaping_test() {
    let bytes = include_bytes!("testfiles/trivial.xml");
    let list = mathmlparser::parse(&bytes[..]).expect("invalid parse");
    println!("{:?}", math_render::list_to_boxes(list));
    //panic!();
}

#[test]
fn tree_test() {
    use math_render::tree::*;
    use std::rc::Rc;
    let sub11 = Node{ name: "sub11".into(), children: vec![] };
    let sub12 = Node{ name: "sub12".into(), children: vec![] };
    let sub1 = Node{ name: "sub1".into(), children: vec![sub11, sub12] };
    let sub2 = Node{ name: "sub2".into(), children: vec![] };
    let root = Node{ name: "root".into(), children: vec![sub1, sub2] };

    let func = Rc::new(combine);
    let result = map_tree(root, func);
    println!("{:?}", result);
    assert!(result == Node { name: "sub11sub12sub2".into(), children: vec![] });
    //panic!()
}

#[test]
fn tree_iter_test() {
    use math_render::tree_iter::*;
    use std::rc::Rc;
    let sub11 = Node{ name: "sub11".into(), children: vec![] };
    let sub12 = Node{ name: "sub12".into(), children: vec![] };
    let sub1 = Node{ name: "sub1".into(), children: vec![sub11, sub12] };
    let sub2 = Node{ name: "sub2".into(), children: vec![] };
    let root = Node{ name: "root".into(), children: vec![sub1, sub2] };

    let func = Rc::new(combine);
    let result = map_tree(root, func);
    println!("{:?}", result);
    assert!(result == Node { name: "root".into(), children: vec![
                            Node { name: "sub1".into(), children: vec![
                                    Node { name: "sub11sub12".into(), children: vec![] }
                            ] },
                            Node { name: "sub2".into(), children: vec![] }
                      ] }
                );
}
