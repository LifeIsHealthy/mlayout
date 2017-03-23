use super::*;

use super::layout::{OperatorProperties, MathLayout};
use types::{MathExpression, Index};
use math_box::{Extents, MathBoxMetrics};

fn indices_of_stretchy_elements<'a>(list: &[Index],
                                    expr: &'a MathExpression,
                                    options: LayoutOptions<'a>)
                                    -> Vec<Index> {
    list.iter()
        .cloned()
        .filter(|&index| {
                    let item = expr.get_item(index);
                    item.operator_properties(expr, options)
                        .and_then(|x| x.stretch_properties)
                        .is_some()
                })
        .collect()
}

pub fn layout_list_element<'a, T>(item: T,
                                  expr: &'a MathExpression,
                                  options: LayoutOptions<'a>)
                                  -> MathBox<'a>
    where T: MathLayout<'a, MathBox<'a>>
{
    if let Some(OperatorProperties {
                    leading_space,
                    trailing_space,
                    ..
                }) = item.operator_properties(expr, options) {
        if options.style.math_style == MathStyle::Display {
            let left_space = MathBox::empty(Extents::new(0, leading_space, 0, 0));
            let mut elem = item.layout(expr, options);
            elem.origin.x += leading_space;
            let mut right_space = MathBox::empty(Extents::new(0, trailing_space, 0, 0));
            right_space.origin.x += leading_space + elem.advance_width();
            return MathBox::with_vec(vec![left_space, elem, right_space]);
        }
    }
    item.layout(expr, options)
}


pub fn layout_strechy_list<'a>(list: &'a [Index],
                               expr: &'a MathExpression,
                               options: LayoutOptions<'a>)
                               -> Box<Iterator<Item = MathBox<'a>> + 'a> {
    let stretchy_indices = indices_of_stretchy_elements(list, expr, options);

    if stretchy_indices.is_empty() {
        return Box::new(list.iter()
                            .filter_map(move |&index| expr.get_item(index))
                            .map(move |item| layout_list_element(item, expr, options)));
    }

    let mut items = Vec::with_capacity(list.len());

    let mut max_intrinsic_size = 0;
    for index in list {
        let item = expr.get_item(*index);
        if let Some(OperatorProperties { stretch_properties: Some(stretch_props), .. }) =
            item.operator_properties(expr, options) {
            max_intrinsic_size = ::std::cmp::max(max_intrinsic_size, stretch_props.intrinsic_size);
            // this is replaced later
            items.push(MathBox::default());
        } else {
            let math_box = layout_list_element(item, expr, options);
            items.push(math_box);
        }
    }

    let max_ascent = items.iter().map(|math_box| math_box.extents().ascent).max();
    let max_descent = items.iter().map(|math_box| math_box.extents().descent).max();

    let options = LayoutOptions {
        stretch_size: Some(Extents {
                               left_side_bearing: 0,
                               width: 0,
                               ascent: max_ascent.unwrap_or_default(),
                               descent: max_descent.unwrap_or_default(),
                           }),
        ..options
    };

    let mut list_iter = list.iter().enumerate();
    for stretchy_index in stretchy_indices.iter() {
        let (insertion_point, index) = list_iter.find(|&(_, index)| index == stretchy_index)
            .unwrap();
        let item = expr.get_item(*index);
        let math_box = layout_list_element(item, expr, options);
        items.insert(insertion_point, math_box);
    }

    Box::new(items.into_iter())
}
