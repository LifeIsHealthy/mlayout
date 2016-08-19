use std::cmp::max;

use types::{GlyphCode, MathStyle, Glyph, CornerPosition};
use types::CornerPosition::{TopLeft, BottomRight};
use super::font::{MathFont, hb, Position};
use super::math_box::{MathBox, Content};

fn get_first_glyph(math_box: &MathBox) -> Option<GlyphCode> {
    match math_box.content {
        Content::Glyph(Glyph { ref glyph_code, .. }) => Some(*glyph_code),
        Content::Boxes(ref list) => get_last_glyph(list.first().unwrap()),
        _ => None,
    }
}

fn get_last_glyph(math_box: &MathBox) -> Option<GlyphCode> {
    match math_box.content {
        Content::Glyph(Glyph { ref glyph_code, .. }) => Some(*glyph_code),
        Content::Boxes(ref list) => get_last_glyph(list.last().unwrap()),
        _ => None,
    }
}

pub fn get_superscript_shift_up(superscript: &MathBox,
                                nucleus: &MathBox,
                                font: &MathFont,
                                style: MathStyle)
                                -> Position {
    let std_shift_up = font.get_math_constant(if style.is_cramped() {
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

pub fn get_subscript_shift_dn(subscript: &MathBox,
                              nucleus: &MathBox,
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

// TODO: needs tests
pub fn get_attachment_kern(nucleus: &MathBox,
                           attachment: &MathBox,
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
        match attachment_position {
            TopLeft | BottomRight => kerning -= font.get_italic_correction(nucleus_glyph) / 2,
            _ => kerning += font.get_italic_correction(nucleus_glyph) / 2,
        }
        let attachment_glyph = if attachment_position.is_left() {
            get_last_glyph(attachment)
        } else {
            get_first_glyph(attachment)
        };
        if let Some(attachment_glyph) = attachment_glyph {
            if attachment_position.is_top() {
                kerning += font.get_math_kern(nucleus_glyph,
                                              attachment_position,
                                              attachment_shift - attachment.ink_extents.descent);
                kerning += font.get_math_kern(attachment_glyph,
                                              attachment_position.diagonal_mirror(),
                                              nucleus.ink_extents.ascent - attachment_shift);
            } else {
                kerning += font.get_math_kern(nucleus_glyph,
                                              attachment_position,
                                              attachment.ink_extents.ascent - attachment_shift);
                kerning += font.get_math_kern(attachment_glyph,
                                              attachment_position.diagonal_mirror(),
                                              attachment_shift - nucleus.ink_extents.descent);
            }
        }
    };
    kerning
}
