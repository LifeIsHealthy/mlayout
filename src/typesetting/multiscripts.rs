use std::cmp::max;

use super::layout::LayoutOptions;
use super::math_box::{MathBox, MathBoxMetrics};
use super::shaper::{MathConstant, Position};
use crate::types::CornerPosition;

pub fn get_superscript_shift_up(
    superscript: &MathBox,
    nucleus: &MathBox,
    options: LayoutOptions,
) -> Position {
    let shaper = options.shaper;
    let style = options.style;
    let std_shift_up = shaper.math_constant(if style.is_cramped {
        MathConstant::SuperscriptShiftUpCramped
    } else {
        MathConstant::SuperscriptShiftUp
    });

    let min_shift_up =
        superscript.extents().descent + shaper.math_constant(MathConstant::SuperscriptBottomMin);

    let min_shift_from_baseline_drop =
        nucleus.extents().ascent - shaper.math_constant(MathConstant::SuperscriptBaselineDropMax);

    max(
        min_shift_from_baseline_drop,
        max(std_shift_up, min_shift_up),
    )
}

pub fn get_subscript_shift_dn(
    subscript: &MathBox,
    nucleus: &MathBox,
    options: LayoutOptions,
) -> Position {
    let shaper = options.shaper;
    let min_shift_dn_from_baseline_drop =
        nucleus.extents().descent + shaper.math_constant(MathConstant::SubscriptBaselineDropMin);

    let std_shift_dn = shaper.math_constant(MathConstant::SubscriptShiftDown);
    let min_shift_dn =
        subscript.extents().ascent - shaper.math_constant(MathConstant::SubscriptTopMax);

    max(
        min_shift_dn_from_baseline_drop,
        max(std_shift_dn, min_shift_dn),
    )
}

pub fn get_subsup_shifts(
    subscript: &MathBox,
    superscript: &MathBox,
    nucleus: &MathBox,
    options: LayoutOptions,
) -> (Position, Position) {
    let (shaper, _style) = (options.shaper, options.style);
    let mut super_shift = get_superscript_shift_up(superscript, nucleus, options);
    let mut sub_shift = get_subscript_shift_dn(subscript, nucleus, options);

    let subsup_gap_min = shaper.math_constant(MathConstant::SubSuperscriptGapMin);
    let super_bottom_max = shaper.math_constant(MathConstant::SuperscriptBottomMaxWithSubscript);

    let super_bottom = super_shift - superscript.extents().descent;
    let sub_top = -sub_shift + subscript.extents().ascent;
    let gap = super_bottom - sub_top;
    if gap < subsup_gap_min {
        let needed_space = subsup_gap_min - gap;
        assert!(needed_space > 0);
        let super_max_additional_shift = super_bottom_max - super_bottom;
        if needed_space <= super_max_additional_shift {
            super_shift += needed_space;
        } else {
            super_shift += super_max_additional_shift;
            sub_shift += needed_space - super_max_additional_shift;
        }
    }

    (sub_shift, super_shift)
}

// TODO: needs tests
pub fn get_attachment_kern(
    nucleus: &MathBox,
    attachment: &MathBox,
    attachment_position: CornerPosition,
    attachment_shift: Position,
    options: LayoutOptions,
) -> Position {
    let shaper = options.shaper;
    let mut kerning = 0;

    let nucleus_glyph = if attachment_position.is_left() {
        nucleus.last_glyph()
    } else {
        nucleus.first_glyph()
    };

    if let Some((nucleus_glyph, scale)) = nucleus_glyph {
        let attachment_glyph = if attachment_position.is_left() {
            attachment.last_glyph()
        } else {
            attachment.first_glyph()
        };
        if let Some((attachment_glyph, attachment_scale)) = attachment_glyph {
            let (bch, ach) = if attachment_position.is_top() {
                let base_correction_height =
                    attachment_shift - attachment.extents().descent * attachment_scale;
                let attachment_correction_height =
                    nucleus.extents().ascent * scale - attachment_shift;
                (base_correction_height, attachment_correction_height)
            } else {
                let base_correction_height =
                    -attachment_shift + attachment.extents().ascent * attachment_scale;
                let attachment_correction_height =
                    attachment_shift - nucleus.extents().descent * scale;
                (base_correction_height, attachment_correction_height)
            };
            kerning += shaper.math_kerning(&nucleus_glyph, attachment_position, bch) * scale;
            kerning += shaper.math_kerning(
                &attachment_glyph,
                attachment_position.diagonal_mirror(),
                ach,
            ) * attachment_scale;
        }
    };
    kerning
}

pub fn position_attachment(
    attachment: &mut MathBox,
    nucleus: &mut MathBox,
    nucleus_is_largeop: bool,
    attachment_position: CornerPosition,
    attachment_vert_shift: i32,
    options: LayoutOptions,
) {
    let shift = attachment_vert_shift;

    let kern = get_attachment_kern(nucleus, attachment, attachment_position, shift, options);

    let italic_correction = match (nucleus_is_largeop, attachment_position.is_top()) {
        (true, false) => -nucleus.italic_correction(),
        (false, true) => nucleus.italic_correction(),
        _ => 0,
    };

    if attachment_position.is_left() {
        attachment.origin.x -= kern;
        unimplemented!();
    } else {
        attachment.origin.x = nucleus.origin.x + nucleus.advance_width() + italic_correction;
        attachment.origin.x += kern;
    }

    attachment.origin.y += nucleus.origin.y;
    if attachment_position.is_top() {
        attachment.origin.y -= shift;
    } else {
        attachment.origin.y += shift;
    }
}
