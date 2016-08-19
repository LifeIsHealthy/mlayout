
pub extern crate harfbuzz_sys as hb;

use std;
use std::marker::PhantomData;

use types::{MathStyle, GlyphCode, CornerPosition};
use super::math_box::{Point, Extents, Bounds};

use super::freetype;
use super::freetype::face;

pub type Position = hb::hb_position_t;
pub type GlyphPosition = hb::hb_glyph_position_t;
pub type GlyphExtents = hb::hb_glyph_extents_t;
pub type GlyphInfo = hb::hb_glyph_info_t;

pub struct Blob<'a> {
    hb_blob: *mut hb::hb_blob_t,
    _marker: PhantomData<&'a [u8]>,
}
impl<'a> Blob<'a> {
    pub fn with_bytes(bytes: &[u8]) -> Blob {
        let hb_blob = unsafe {
            hb::hb_blob_create(bytes.as_ptr() as *const i8,
                               bytes.len() as u32,
                               hb::HB_MEMORY_MODE_READONLY,
                               0 as *mut _,
                               None)
        };
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }

    pub fn from_raw<'b>(blob: *mut hb::hb_blob_t) -> Blob<'b> {
        Blob {
            hb_blob: blob,
            _marker: PhantomData,
        }
    }

    pub fn get_data(&self) -> &'a [u8] {
        unsafe {
            let mut length = hb::hb_blob_get_length(self.hb_blob);
            let data_ptr = hb::hb_blob_get_data(self.hb_blob, &mut length as *mut _);
            std::slice::from_raw_parts(data_ptr as *const u8, length as usize)
        }
    }
}

impl<'a> Clone for Blob<'a> {
    fn clone(&self) -> Self {
        let hb_blob = unsafe { hb::hb_blob_reference(self.hb_blob) };
        Blob {
            hb_blob: hb_blob,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a> Send for Blob<'a> {}
unsafe impl<'a> Sync for Blob<'a> {}

impl<'a> Drop for Blob<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_blob_destroy(self.hb_blob);
        }
    }
}

#[derive(Debug)]
pub struct MathFont<'a> {
    pub hb_font: *mut hb::hb_font_t,
    ft_face: freetype::Face<'a>,
}

impl<'a> MathFont<'a> {
    pub fn from_bytes<'b>(bytes: &'b [u8],
                          face_index: isize,
                          ft_library: &freetype::Library)
                          -> MathFont<'b> {
        let blob = Blob::with_bytes(bytes);
        let hb_font = unsafe {
            let hb_face = hb::hb_face_create(blob.hb_blob, face_index as u32);
            let hb_font = hb::hb_font_create(hb_face);
            hb::hb_face_destroy(hb_face);
            hb::hb_ot_font_set_funcs(hb_font);
            hb_font
        };
        let face = ft_library.new_memory_face(blob.get_data(), face_index).unwrap();
        MathFont {
            hb_font: hb_font,
            ft_face: face,
        }
    }

    pub fn from_raw<'b, 'c>(font: *mut hb::hb_font_t,
                            ft_library: &'c freetype::Library)
                            -> MathFont<'b> {
        let (blob, index) = unsafe {
            let hb_face = hb::hb_font_get_face(font);
            let index = hb::hb_face_get_index(hb_face);
            let blob = Blob::from_raw(hb::hb_face_reference_blob(hb_face));
            (blob, index)
        };
        let face = ft_library.new_memory_face(blob.get_data(), index as isize).unwrap();
        MathFont {
            hb_font: font,
            ft_face: face,
        }
    }

    pub fn get_glyph_h_advance(&self, codepoint: GlyphCode) -> i32 {
        unsafe { hb::hb_font_get_glyph_h_advance(self.hb_font, codepoint) }
    }
    pub fn get_glyph_v_advance(&self, codepoint: GlyphCode) -> i32 {
        unsafe { hb::hb_font_get_glyph_v_advance(self.hb_font, codepoint) }
    }
    pub fn get_glyph_bounds(&self, codepoint: GlyphCode) -> Bounds {
        self.ft_face.load_glyph(codepoint, face::NO_SCALE).unwrap();
        let metrics = self.ft_face.glyph().metrics();
        let extents = Extents {
            width: metrics.width as i32,
            ascent: metrics.horiBearingY as i32,
            descent: metrics.height as i32 - metrics.horiBearingY as i32,
        };
        let pos = Point { x: metrics.horiBearingX as i32, y: 0 };
        Bounds { extents: extents, origin: pos }
    }
    pub fn get_math_table(&self) -> Blob {
        let hb_blob = unsafe {
            let face = hb::hb_font_get_face(self.hb_font);
            hb::hb_face_reference_table(face, ot_tag!('M', 'A', 'T', 'H'))
        };
        Blob::from_raw(hb_blob)
    }

    pub fn get_math_constant(&self, index: hb::hb_ot_math_constant_t) -> i32 {
        unsafe { hb::hb_ot_layout_get_math_constant(self.hb_font, index) }
    }

    pub fn get_math_kern(&self,
                         glyph: GlyphCode,
                         corner: CornerPosition,
                         correction_height: Position)
                         -> Position {
        unsafe {
            hb::hb_ot_layout_get_math_kerning(self.hb_font, glyph, corner as hb::hb_ot_math_kern_t, correction_height)
        }
    }

    pub fn get_italic_correction(&self, glyph: GlyphCode) -> Position {
        unsafe {
            hb::hb_ot_layout_get_math_italic_correction(self.hb_font, glyph)
        }
    }

    pub fn get_top_accent_attachment(&self, glyph: GlyphCode) -> Position {
        unsafe {
            hb::hb_ot_layout_get_math_top_accent_attachment(self.hb_font, glyph)
        }
    }

    pub fn scale_factor_for_style(&self, style: MathStyle) -> i32 {
        if style <= MathStyle::ScriptStyle {
            if style <= MathStyle::ScriptScriptStyle {
                self.get_math_constant(hb::HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN)
            } else {
                self.get_math_constant(hb::HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN)
            }
        } else {
            100
        }
    }
}

impl<'a> Clone for MathFont<'a> {
    fn clone(&self) -> Self {
        let hb_font = unsafe { hb::hb_font_reference(self.hb_font) };
        let ft_face = self.ft_face.clone();
        MathFont {
            hb_font: hb_font,
            ft_face: ft_face,
        }
    }
}

unsafe impl<'a> Send for MathFont<'a> {}
unsafe impl<'a> Sync for MathFont<'a> {}

impl<'a> Drop for MathFont<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_font_destroy(self.hb_font);
        }
    }
}
