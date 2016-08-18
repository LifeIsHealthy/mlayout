
pub extern crate harfbuzz_sys as hb;

use std;
use std::iter::Iterator;

use types::{Glyph, MathStyle};
use super::math_box::{MathBox, Content};
use super::font::{MathFont, Codepoint, GlyphPosition, GlyphInfo};

const HB_SCRIPT_MATH: u32 = ot_tag!('M', 'a', 't', 'h');

#[derive(Debug)]
struct Buffer {
    hb_buffer: *mut hb::hb_buffer_t,
}
impl Buffer {
    fn new() -> Buffer {
        let buffer = unsafe {
            let buffer = hb::hb_buffer_create();
            hb::hb_buffer_set_script(buffer, HB_SCRIPT_MATH);
            hb::hb_buffer_set_language(buffer, hb::hb_language_get_default());
            buffer
        };

        Buffer { hb_buffer: buffer }
    }

    fn add_str(&mut self, string: &str) {
        let chars = string.as_ptr() as *const i8;
        unsafe {
            hb::hb_buffer_add_utf8(self.hb_buffer,
                                   chars,
                                   string.len() as i32,
                                   0,
                                   string.len() as i32);
        }
    }

    fn set_direction(&mut self, direction: hb::hb_direction_t) {
        unsafe {
            hb::hb_buffer_set_direction(self.hb_buffer, direction);
        }
    }

    fn clear(&mut self) {
        unsafe {
            hb::hb_buffer_clear_contents(self.hb_buffer);
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            hb::hb_buffer_destroy(self.hb_buffer);
        }
    }
}

fn shape<'a>(font: &MathFont,
             buffer: &'a Buffer,
             style: MathStyle)
             -> (&'a mut [GlyphPosition], &'a mut [GlyphInfo]) {
    unsafe {
        let mut features: Vec<hb::hb_feature_t> = Vec::with_capacity(1);
        if style <= MathStyle::ScriptStyle {
            let math_variants_tag = ot_tag!('s', 's', 't', 'y');
            let mut variant_num = 1;
            if style <= MathStyle::ScriptScriptStyle {
                variant_num = 2;
            }
            features.push(hb::hb_feature_t {
                tag: math_variants_tag,
                value: variant_num,
                start: 0,
                end: std::u32::MAX,
            })
        }
        hb::hb_shape(font.hb_font,
                     buffer.hb_buffer,
                     features.as_ptr(),
                     features.len() as u32);


        let mut length: u32 = 0;

        let glyph_info = hb::hb_buffer_get_glyph_infos(buffer.hb_buffer, &mut length as *mut u32);
        let glyph_pos = hb::hb_buffer_get_glyph_positions(buffer.hb_buffer,
                                                          &mut length as *mut u32);

        let positions = std::slice::from_raw_parts_mut(glyph_pos, length as usize);
        let infos = std::slice::from_raw_parts_mut(glyph_info, length as usize);

        (positions, infos)
    }

}

fn box_from_glyph(font: &MathFont, glyph: Codepoint, style: MathStyle) -> MathBox {
    let scale = font.scale_factor_for_style(style);
    let content = Content::Glyph(Glyph {
        glyph_code: glyph,
        scale_x: scale,
        scale_y: scale,
    });
    let bounds = font.get_glyph_bounds(glyph);
    let italic_correction = font.get_italic_correction(glyph);
    let mut logical_extents = bounds.extents;
    logical_extents.width = font.get_glyph_h_advance(glyph);
    //if italic_correction == 0 {
    //    italic_correction = font.get_glyph_h_advance(glyph) - bounds.extents.width -
    //                        bounds.origin.x;
    //}
    MathBox {
        origin: bounds.origin * scale / 100,
        ink_extents: bounds.extents * scale / 100,
        logical_extents: logical_extents * scale / 100,
        italic_correction: italic_correction,
        top_accent_attachment: font.get_top_accent_attachment(glyph),
        content: content,
    }
}


pub struct MathShaper {
    buffer: Buffer,
}

impl MathShaper {
    pub fn new() -> MathShaper {
        MathShaper { buffer: Buffer::new() }
    }

    pub fn shape(&mut self, string: &str, font: &MathFont, style: MathStyle) -> Vec<MathBox> {
        self.buffer.clear();
        self.buffer.set_direction(hb::HB_DIRECTION_LTR);
        unsafe {
            hb::hb_buffer_guess_segment_properties(self.buffer.hb_buffer);
        }
        self.buffer.add_str(string);
        let (positions, infos) = shape(font, &self.buffer, style);
        let mut cursor = 0i32;
        let list_iter = positions.iter().zip(infos.iter()).map(move |pos_info| {
            let pos = pos_info.0;
            let info = pos_info.1;
            let mut new_box = box_from_glyph(font, info.codepoint, style);

            if let Content::Glyph(Glyph { scale_x, .. }) = new_box.content {
                let advance_width = pos.x_advance * scale_x / 100;
                new_box.origin.x = cursor;
                // new_box.logical_extents.width = advance_width;
                cursor += advance_width;
                new_box
            } else {
                unreachable!();
            }
        });
        list_iter.collect()
    }
}
