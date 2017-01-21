
pub extern crate harfbuzz_sys as hb;

use std;
use std::cell::RefCell;
use std::marker::PhantomData;

use types::{Glyph, GlyphCode, CornerPosition, PercentScale};
use super::math_box::{Point, Extents, Bounds};

use super::freetype;
use super::freetype::face;

/// Type for a coordinate that represents some metric of a font (e.g. advance width).
pub type Position = hb::hb_position_t;
/// Wrapper of the `hb_glyph_info_t` struct.
pub type GlyphPosition = hb::hb_glyph_position_t;
/// Metrics of a glyph.
pub type GlyphExtents = hb::hb_glyph_extents_t;
/// Wrapper of the `hb_glyph_info_t` struct.
pub type GlyphInfo = hb::hb_glyph_info_t;

/// A wrapper around the harfbuzz `hb_blob_t`. It owns a slice of bytes and is fully threadsafe.
/// A blob includes reference counting to allow shared usage of its memory.
pub struct Blob<'a> {
    hb_blob: *mut hb::hb_blob_t,
    _marker: PhantomData<&'a [u8]>,
}
impl<'a> Blob<'a> {
    /// Create from slice.
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

    /// Make a `Blob` from a raw harfbuzz pointer. Transfers ownership.
    pub fn from_raw<'b>(blob: *mut hb::hb_blob_t) -> Blob<'b> {
        Blob {
            hb_blob: blob,
            _marker: PhantomData,
        }
    }

    /// Convert the `Blob` into a raw harfbuzz pointer.
    pub fn as_raw(&self) -> *mut hb::hb_blob_t {
        self.hb_blob
    }

    /// Get a slice of the `Blob`'s bytes.
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

/// The basic font structure used
#[derive(Debug)]
pub struct MathFont<'a> {
    pub hb_font: *mut hb::hb_font_t,
    pub ft_face: RefCell<freetype::Face<'a>>,
}

impl<'a> MathFont<'a> {
    /// Create a `MathFont` from raw bytes of an opentype font file.
    ///
    /// # Arguments
    ///
    /// * `bytes` – The bytes of an opentype font file
    /// * `face_index` – The face_index of the font face inside the file
    /// * `ft_library` – A freetype `Library` object to use for initialization
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
            ft_face: RefCell::new(face),
        }
    }

    pub unsafe fn from_raw<'b, 'c>(font: *mut hb::hb_font_t,
                                   ft_library: &'c freetype::Library)
                                   -> MathFont<'b> {
        let hb_face = hb::hb_font_get_face(font);
        let index = hb::hb_face_get_index(hb_face);
        let blob = Blob::from_raw(hb::hb_face_reference_blob(hb_face));

        let face = ft_library.new_memory_face(blob.get_data(), index as isize).unwrap();
        MathFont {
            hb_font: font,
            ft_face: RefCell::new(face),
        }
    }

    pub fn get_glyph_h_advance(&self, glyph: Glyph) -> i32 {
        let unscaled = unsafe { hb::hb_font_get_glyph_h_advance(self.hb_font, glyph.glyph_code) };
        unscaled * glyph.scale.horiz
    }
    pub fn get_glyph_v_advance(&self, glyph: Glyph) -> i32 {
        let unscaled = unsafe { hb::hb_font_get_glyph_v_advance(self.hb_font, glyph.glyph_code) };
        unscaled * glyph.scale.vert
    }
    pub fn get_glyph_bounds(&self, glyph: Glyph) -> Bounds {
        let result = self.ft_face.borrow().load_glyph(glyph.glyph_code, face::NO_SCALE);
        if result.is_err() {
            let new_glyph_index = self.ft_face.borrow().get_char_index(0x221A);
            println!("{:?}    {:?}", glyph.glyph_code, new_glyph_index);
            self.ft_face
                .borrow()
                .load_glyph(new_glyph_index, face::NO_SCALE)
                .expect("freetype could not load glyph");
        }
        let metrics = self.ft_face.borrow().glyph().metrics();
        let extents = Extents {
            width: metrics.width as i32,
            ascent: metrics.horiBearingY as i32,
            descent: metrics.height as i32 - metrics.horiBearingY as i32,
        };
        let extents = extents * glyph.scale;
        let pos = Point { x: 0, y: 0 };
        Bounds {
            extents: extents,
            origin: pos,
        }
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
                         glyph: Glyph,
                         corner: CornerPosition,
                         correction_height: Position)
                         -> Position {
        let unscaled = unsafe {
            hb::hb_ot_layout_get_math_kerning(self.hb_font,
                                              glyph.glyph_code,
                                              corner as hb::hb_ot_math_kern_t,
                                              correction_height / glyph.scale.vert)
        };
        unscaled * glyph.scale.horiz
    }

    pub fn get_italic_correction(&self, glyph: Glyph) -> Position {
        let unscaled =
            unsafe { hb::hb_ot_layout_get_math_italic_correction(self.hb_font, glyph.glyph_code) };
        unscaled * glyph.scale.horiz
    }

    pub fn get_top_accent_attachment(&self, glyph: Glyph) -> Position {
        let unscaled = unsafe {
            hb::hb_ot_layout_get_math_top_accent_attachment(self.hb_font, glyph.glyph_code)
        };
        unscaled * glyph.scale.horiz
    }

    pub fn get_glyph_name(&self, glyph: GlyphCode) -> String {
        let string_capacity = 512;
        let mut buffer: Vec<u8> = Vec::with_capacity(string_capacity);
        let ptr = buffer.as_mut_ptr();
        unsafe {
            freetype::ffi::FT_Get_Glyph_Name(self.ft_face.borrow_mut().raw_mut() as *mut _,
                                             glyph,
                                             ptr as *mut _,
                                             string_capacity as u32);
        }
        let cstr = unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr() as *const _) };
        unsafe { buffer.set_len(cstr.to_bytes().len()) };
        String::from_utf8(buffer).unwrap()
    }

    pub fn scale_factor_for_script_level(&self, script_level: u8) -> PercentScale {
        let percent = if script_level >= 1 {
            if script_level >= 2 {
                self.get_math_constant(hb::HB_OT_MATH_CONSTANT_SCRIPT_SCRIPT_PERCENT_SCALE_DOWN)
            } else {
                self.get_math_constant(hb::HB_OT_MATH_CONSTANT_SCRIPT_PERCENT_SCALE_DOWN)
            }
        } else {
            100
        };
        PercentScale::new(percent as u8)
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

impl<'a> Drop for MathFont<'a> {
    fn drop(&mut self) {
        unsafe {
            hb::hb_font_destroy(self.hb_font);
        }
    }
}
