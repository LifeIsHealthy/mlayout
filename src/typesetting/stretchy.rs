use super::*;

use super::layout::{MathLayout, OperatorProperties};
use crate::types::MathExpression;
use crate::math_box::{Extents, MathBoxMetrics};

fn indices_of_stretchy_elements(list: &[MathExpression], options: LayoutOptions) -> Vec<usize> {
    list.iter()
        .enumerate()
        .filter(|&(_, ref expr)| expr.can_stretch(options))
        .map(|(index, _)| index)
        .collect()
}

pub fn layout_list_element<T: MathLayout>(item: &T, options: LayoutOptions) -> MathBox {
    if let Some(OperatorProperties {
        leading_space,
        trailing_space,
        ..
    }) = item.operator_properties(options)
    {
        if options.style.math_style == MathStyle::Display {
            let left_space = MathBox::empty(Extents::new(0, leading_space, 0, 0));
            let mut elem = item.layout(options);
            elem.origin.x += leading_space;
            let mut right_space = MathBox::empty(Extents::new(0, trailing_space, 0, 0));
            right_space.origin.x += leading_space + elem.advance_width();
            return MathBox::with_vec(vec![left_space, elem, right_space]);
        }
    }
    item.layout(options)
}

pub(crate) fn layout_strechy_list(list: &[MathExpression], options: LayoutOptions) -> Vec<MathBox> {
    let stretchy_indices = indices_of_stretchy_elements(list, options);

    if stretchy_indices.is_empty() {
        return list.iter()
            .map(move |item| layout_list_element(item, options))
            .collect();
    }

    let mut items = Vec::with_capacity(list.len());

    let mut max_intrinsic_size = 0;
    for ref item in list {
        if let Some(OperatorProperties {
            stretch_properties: Some(stretch_props),
            ..
        }) = item.operator_properties(options)
        {
            max_intrinsic_size = ::std::cmp::max(max_intrinsic_size, stretch_props.intrinsic_size);
        } else {
            let math_box = layout_list_element(*item, options);
            items.push(math_box);
        }
    }

    let max_ascent = items.iter().map(|math_box| math_box.extents().ascent).max();
    let max_descent = items
        .iter()
        .map(|math_box| math_box.extents().descent)
        .max();

    let options = LayoutOptions {
        stretch_size: Some(Extents {
            left_side_bearing: 0,
            width: 0,
            ascent: max_ascent.unwrap_or_default(),
            descent: max_descent.unwrap_or_default(),
        }),
        ..options
    };

    for &stretchy_index in stretchy_indices.iter() {
        let stretchy_item = &list[stretchy_index];
        let math_box = layout_list_element(stretchy_item, options);
        items.insert(stretchy_index, math_box);
    }

    items
}

// TODO: Tests