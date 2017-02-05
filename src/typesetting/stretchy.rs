use super::*;

use super::layout::OperatorProperties;
use std::fmt::Debug;
use types::MathExpression;
use math_box::Extents;

fn indices_of_stretchy_elements<'a, T: Debug>(list: &[MathExpression<T>],
                                              options: LayoutOptions<'a>)
                                              -> Vec<usize> {
    list.iter()
        .enumerate()
        .filter_map(|(index, elem)| {
            elem.operator_properties(options).and_then(|x| x.stretch_properties).map(|_| index)
        })
        .collect()
}

pub fn layout_list_element<'a, T: 'a + Debug>(item: MathExpression<T>,
                                              options: LayoutOptions<'a>)
                                              -> MathBox<'a, T> {
    if let Some(OperatorProperties { leading_space, trailing_space, .. }) =
        item.operator_properties(options) {
        if options.style.math_style == MathStyle::Display {
            let left_space = MathBox::empty(Extents::new(leading_space, 0, 0));
            let mut elem = item.layout(options);
            elem.origin.x += leading_space;
            let mut right_space = MathBox::empty(Extents::new(trailing_space, 0, 0));
            right_space.origin.x += leading_space + elem.width();
            return MathBox::with_vec(vec![left_space, elem, right_space]);
        }
    }
    item.layout(options)
}


pub fn layout_strechy_list<'a, T: 'a + Debug>(list: Vec<MathExpression<T>>,
                                              options: LayoutOptions<'a>)
                                              -> Box<Iterator<Item = MathBox<'a, T>> + 'a> {
    let stretchy_indices = indices_of_stretchy_elements(&list, options);

    if stretchy_indices.is_empty() {
        return Box::new(list.into_iter().map(move |item| layout_list_element(item, options)));
    }

    let mut stretchy_elems = Vec::with_capacity(stretchy_indices.len());
    let mut non_stretchy_elems = Vec::with_capacity(list.len());

    let mut max_intrinsic_size = 0;
    for elem in list {
        if let Some(OperatorProperties { stretch_properties: Some(stretch_props), .. }) =
            elem.operator_properties(options) {
            stretchy_elems.push(elem);
            max_intrinsic_size = ::std::cmp::max(max_intrinsic_size, stretch_props.intrinsic_size);
        } else {
            let math_box = layout_list_element(elem, options);
            non_stretchy_elems.push(math_box);
        }
    }

    assert_eq!(stretchy_indices.len(), stretchy_elems.len());

    let max_ascent = non_stretchy_elems.iter().map(|math_box| math_box.ascent()).max();
    let max_descent = non_stretchy_elems.iter().map(|math_box| math_box.descent()).max();

    let options = LayoutOptions {
        stretch_size: Some(Extents {
            width: 0,
            ascent: max_ascent.unwrap_or_default(),
            descent: max_descent.unwrap_or_default(),
        }),
        ..options
    };

    for (index, stretchy) in stretchy_indices.iter().zip(stretchy_elems.into_iter()) {
        let math_box = layout_list_element(stretchy, options);
        non_stretchy_elems.insert(*index, math_box);
    }

    Box::new(non_stretchy_elems.into_iter())
}
