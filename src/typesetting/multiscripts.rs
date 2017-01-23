use std::cmp::max;

use types::{LayoutStyle, CornerPosition};
use super::shaper::{MathShaper, MathConstant, Position};
use super::math_box::MathBox;

pub fn get_superscript_shift_up<'a, T: 'a>(superscript: &MathBox<'a, T>,
                                   nucleus: &MathBox<'a, T>,
                                   shaper: &MathShaper,
                                   style: LayoutStyle)
                                   -> Position {
    let std_shift_up = shaper.math_constant(if style.is_cramped {
        MathConstant::SuperscriptShiftUpCramped
    } else {
        MathConstant::SuperscriptShiftUp
    });

    let min_shift_up = superscript.descent() +
                       shaper.math_constant(MathConstant::SuperscriptBottomMin);

    let min_shift_from_baseline_drop =
        nucleus.ascent() - shaper.math_constant(MathConstant::SuperscriptBaselineDropMax);



    max(min_shift_from_baseline_drop,
        max(std_shift_up, min_shift_up))
}

pub fn get_subscript_shift_dn<'a, T: 'a>(subscript: &MathBox<'a, T>,
                                 nucleus: &MathBox<'a, T>,
                                 shaper: &MathShaper)
                                 -> Position {
    let min_shift_dn_from_baseline_drop =
        nucleus.descent() + shaper.math_constant(MathConstant::SubscriptBaselineDropMin);

    let std_shift_dn = shaper.math_constant(MathConstant::SubscriptShiftDown);
    let min_shift_dn = subscript.ascent() - shaper.math_constant(MathConstant::SubscriptTopMax);

    max(min_shift_dn_from_baseline_drop,
        max(std_shift_dn, min_shift_dn))
}

pub fn get_subsup_shifts<'a, T: 'a>(subscript: &MathBox<'a, T>,
                            superscript: &MathBox<'a, T>,
                            nucleus: &MathBox<'a, T>,
                            shaper: &MathShaper,
                            style: LayoutStyle)
                            -> (Position, Position) {
    let mut super_shift = get_superscript_shift_up(superscript, nucleus, shaper, style);
    let mut sub_shift = get_subscript_shift_dn(subscript, nucleus, shaper);

    let subsup_gap_min = shaper.math_constant(MathConstant::SubSuperscriptGapMin);
    let super_bottom_max = shaper.math_constant(MathConstant::SuperscriptBottomMaxWithSubscript);

    let super_bottom = super_shift - superscript.descent();
    let sub_top = -sub_shift + subscript.ascent();
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
pub fn get_attachment_kern<'a, T: 'a>(nucleus: &MathBox<'a, T>,
                                      attachment: &MathBox<'a, T>,
                                      attachment_position: CornerPosition,
                                      attachment_shift: Position,
                                      shaper: &MathShaper)
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
                let base_correction_height = attachment_shift - attachment.descent();
                let attachment_correction_height = nucleus.ascent() - attachment_shift;
                (base_correction_height, attachment_correction_height)
            } else {
                let base_correction_height = -attachment_shift + attachment.ascent();
                let attachment_correction_height = attachment_shift - nucleus.descent();
                (base_correction_height, attachment_correction_height)
            };
            kerning += shaper.math_kerning(nucleus_glyph, attachment_position, bch);
            kerning +=
                shaper.math_kerning(attachment_glyph, attachment_position.diagonal_mirror(), ach);
        }
    };
    kerning
}
