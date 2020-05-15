use std::cmp::max;

use types::{LayoutStyle, CornerPosition};
use super::shaper::{MathShaper, MathConstant, Position};
use super::math_box::{MathBox, MathBoxMetrics};

pub fn get_superscript_shift_up<'a>(superscript: &MathBox<'a>,
                                    nucleus: &MathBox<'a>,
                                    shaper: &dyn MathShaper,
                                    style: LayoutStyle)
                                    -> Position {
    let std_shift_up = shaper.math_constant(if style.is_cramped {
                                                MathConstant::SuperscriptShiftUpCramped
                                            } else {
                                                MathConstant::SuperscriptShiftUp
                                            });

    let min_shift_up = superscript.extents().descent +
                       shaper.math_constant(MathConstant::SuperscriptBottomMin);

    let min_shift_from_baseline_drop =
        nucleus.extents().ascent - shaper.math_constant(MathConstant::SuperscriptBaselineDropMax);



    max(min_shift_from_baseline_drop,
        max(std_shift_up, min_shift_up))
}

pub fn get_subscript_shift_dn<'a>(subscript: &MathBox<'a>,
                                  nucleus: &MathBox<'a>,
                                  shaper: &dyn MathShaper)
                                  -> Position {
    let min_shift_dn_from_baseline_drop =
        nucleus.extents().descent + shaper.math_constant(MathConstant::SubscriptBaselineDropMin);

    let std_shift_dn = shaper.math_constant(MathConstant::SubscriptShiftDown);
    let min_shift_dn = subscript.extents().ascent -
                       shaper.math_constant(MathConstant::SubscriptTopMax);

    max(min_shift_dn_from_baseline_drop,
        max(std_shift_dn, min_shift_dn))
}

pub fn get_subsup_shifts<'a>(subscript: &MathBox<'a>,
                             superscript: &MathBox<'a>,
                             nucleus: &MathBox<'a>,
                             shaper: &dyn MathShaper,
                             style: LayoutStyle)
                             -> (Position, Position) {
    let mut super_shift = get_superscript_shift_up(superscript, nucleus, shaper, style);
    let mut sub_shift = get_subscript_shift_dn(subscript, nucleus, shaper);

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
pub fn get_attachment_kern<'a>(nucleus: &MathBox<'a>,
                               attachment: &MathBox<'a>,
                               attachment_position: CornerPosition,
                               attachment_shift: Position,
                               shaper: &dyn MathShaper)
                               -> Position {
    let mut kerning = 0;

    let nucleus_glyph = if attachment_position.is_left() {
        nucleus.last_glyph()
    } else {
        nucleus.first_glyph()
    };

    if let Some(nucleus_glyph) = nucleus_glyph {
        let attachment_glyph = if attachment_position.is_left() {
            attachment.last_glyph()
        } else {
            attachment.first_glyph()
        };
        if let Some(attachment_glyph) = attachment_glyph {
            let (bch, ach) = if attachment_position.is_top() {
                let base_correction_height = attachment_shift - attachment.extents().descent;
                let attachment_correction_height = nucleus.extents().ascent - attachment_shift;
                (base_correction_height, attachment_correction_height)
            } else {
                let base_correction_height = -attachment_shift + attachment.extents().ascent;
                let attachment_correction_height = attachment_shift - nucleus.extents().descent;
                (base_correction_height, attachment_correction_height)
            };
            let bch = bch / nucleus_glyph.scale;
            let ach = ach / attachment_glyph.scale;
            kerning += shaper.math_kerning(nucleus_glyph.glyph_code, attachment_position, bch);
            kerning += shaper.math_kerning(attachment_glyph.glyph_code,
                                           attachment_position.diagonal_mirror(),
                                           ach);
        }
    };
    kerning
}

pub fn position_attachment<'a>(attachment: &mut MathBox<'a>,
                               nucleus: &mut MathBox<'a>,
                               nucleus_is_largeop: bool,
                               attachment_position: CornerPosition,
                               attachment_vert_shift: i32,
                               shaper: &dyn MathShaper) {
    let shift = attachment_vert_shift;

    let kern = get_attachment_kern(nucleus, attachment, attachment_position, shift, shaper);

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
