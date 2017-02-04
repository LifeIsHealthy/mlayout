use super::*;

use super::layout::StretchSize;
use std::fmt::Debug;
use types::MathExpression;

fn indices_of_stretchy_elements<'a, T: Debug>(list: &[MathExpression<T>],
                                              options: LayoutOptions<'a>)
                                              -> Vec<usize> {
    list.iter()
        .enumerate()
        .filter_map(|(index, elem)| elem.stretch_properties(options).map(|_| index))
        .collect()
}


pub fn layout_strechy_list<'a, T: 'a + Debug>(list: Vec<MathExpression<T>>,
                                              options: LayoutOptions<'a>)
                                              -> Box<Iterator<Item = MathBox<'a, T>> + 'a> {
    let stretchy_indices = indices_of_stretchy_elements(&list, options);

    if stretchy_indices.is_empty() {
        let iter = list.into_iter().map(move |item| item.layout(options));
        return Box::new(iter);
    }

    let mut stretchy_elems = Vec::with_capacity(stretchy_indices.len());
    let mut non_stretchy_elems = Vec::with_capacity(list.len());

    let mut max_intrinsic_size = 0;
    for elem in list {
        if let Some(size) = elem.stretch_properties(options) {
            stretchy_elems.push(elem);
            max_intrinsic_size = ::std::cmp::max(max_intrinsic_size, size.intrinsic_size);
        } else {
            let math_box = elem.layout(options);
            non_stretchy_elems.push(math_box);
        }
    }

    assert_eq!(stretchy_indices.len(), stretchy_elems.len());

    let max_ascent = non_stretchy_elems.iter().map(|math_box| math_box.ascent()).max();
    let max_descent = non_stretchy_elems.iter().map(|math_box| math_box.descent()).max();

    let options = LayoutOptions {
        stretch_size: Some(StretchSize {
            ascent: max_ascent.unwrap_or_default(),
            descent: max_descent.unwrap_or_default(),
        }),
        ..options
    };

    for (index, stretchy) in stretchy_indices.iter().zip(stretchy_elems.into_iter()) {
        non_stretchy_elems.insert(*index, stretchy.layout(options));
    }

    Box::new(non_stretchy_elems.into_iter())
}
