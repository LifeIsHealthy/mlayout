use std::cmp::max;

use types::{LayoutStyle, Glyph, CornerPosition};
use super::font::{MathFont, hb, Position};
use super::math_box::{MathBox, Content};

fn get_first_glyph<T>(math_box: &MathBox<T>) -> Option<Glyph> {
    match math_box.content {
        Content::Glyph(glyph) => Some(glyph),
        Content::Boxes(ref list) => get_first_glyph(list.first().unwrap()),
        _ => None,
    }
}

fn get_last_glyph<T>(math_box: &MathBox<T>) -> Option<Glyph> {
    match math_box.content {
        Content::Glyph(glyph) => Some(glyph),
        Content::Boxes(ref list) => get_last_glyph(list.last().unwrap()),
        _ => None,
    }
}

pub fn get_superscript_shift_up<T>(superscript: &MathBox<T>,
                                   nucleus: &MathBox<T>,
                                   font: &MathFont,
                                   style: LayoutStyle)
                                   -> Position {
    let std_shift_up = font.get_math_constant(if style.is_cramped {
        hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP_CRAMPED
    } else {
        hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_SHIFT_UP
    });

    let min_shift_up = superscript.ink_extents.descent +
                       font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_BOTTOM_MIN);

    let min_shift_from_baseline_drop =
        nucleus.ink_extents.ascent -
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_BASELINE_DROP_MAX);



    max(min_shift_from_baseline_drop,
        max(std_shift_up, min_shift_up))
}

pub fn get_subscript_shift_dn<T>(subscript: &MathBox<T>,
                                 nucleus: &MathBox<T>,
                                 font: &MathFont)
                                 -> Position {
    let min_shift_dn_from_baseline_drop =
        nucleus.ink_extents.descent +
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_BASELINE_DROP_MIN);

    let std_shift_dn = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_SHIFT_DOWN);
    let min_shift_dn = subscript.ink_extents.ascent -
                       font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUBSCRIPT_TOP_MAX);

    max(min_shift_dn_from_baseline_drop,
        max(std_shift_dn, min_shift_dn))
}

pub fn get_subsup_shifts<T>(subscript: &MathBox<T>,
                            superscript: &MathBox<T>,
                            nucleus: &MathBox<T>,
                            font: &MathFont,
                            style: LayoutStyle)
                            -> (Position, Position) {
    let mut super_shift = get_superscript_shift_up(superscript, nucleus, font, style);
    let mut sub_shift = get_subscript_shift_dn(subscript, nucleus, font);

    let subsup_gap_min = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUB_SUPERSCRIPT_GAP_MIN);
    let super_bottom_max =
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SUPERSCRIPT_BOTTOM_MAX_WITH_SUBSCRIPT);

    let super_bottom = super_shift - superscript.ink_extents.descent;
    let sub_top = -sub_shift + subscript.ink_extents.ascent;
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
pub fn get_attachment_kern<T>(nucleus: &MathBox<T>,
                              attachment: &MathBox<T>,
                              attachment_position: CornerPosition,
                              attachment_shift: Position,
                              font: &MathFont)
                              -> Position {
    let mut kerning = 0;

    let nucleus_glyph = if attachment_position.is_left() {
        get_last_glyph(nucleus)
    } else {
        get_first_glyph(nucleus)
    };

    if let Some(nucleus_glyph) = nucleus_glyph {
        let attachment_glyph = if attachment_position.is_left() {
            get_last_glyph(attachment)
        } else {
            get_first_glyph(attachment)
        };
        if let Some(attachment_glyph) = attachment_glyph {
            let (bch, ach) = if attachment_position.is_top() {
                let base_correction_height = attachment_shift - attachment.ink_extents.descent;
                let attachment_correction_height = nucleus.ink_extents.ascent - attachment_shift;
                (base_correction_height, attachment_correction_height)
            } else {
                let base_correction_height = -attachment_shift + attachment.ink_extents.ascent;
                let attachment_correction_height = attachment_shift - nucleus.ink_extents.descent;
                (base_correction_height, attachment_correction_height)
            };
            kerning += font.get_math_kern(nucleus_glyph, attachment_position, bch);
            kerning +=
                font.get_math_kern(attachment_glyph, attachment_position.diagonal_mirror(), ach);
        }
    };
    kerning
}
