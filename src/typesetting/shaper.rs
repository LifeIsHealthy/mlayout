
pub extern crate harfbuzz_sys as hb;

use std;
use std::iter::Iterator;
use std::cell::RefCell;

use types::{Glyph, LayoutStyle, PercentScale2D};
use super::math_box::{MathBox, Content, Point};
use super::font::{MathFont, Position, GlyphPosition, GlyphInfo};

const HB_SCRIPT_MATH: u32 = ot_tag!('M', 'a', 't', 'h');

#[allow(dead_code)]
fn ot_tag_to_string(tag: hb::hb_tag_t) -> String {
    let string = String::with_capacity(4);
    let ptr = string.as_ptr() as *mut _;
    unsafe {
        hb::hb_tag_to_string(tag, ptr);
    }
    string
}

fn language_to_string(tag: hb::hb_language_t) -> &'static str {
    let lang_string_ptr = unsafe { hb::hb_language_to_string(tag) };
    let cstring = unsafe { std::ffi::CStr::from_ptr(lang_string_ptr) };
    cstring.to_str().expect("harfbuzz error: language string is not valid utf-8!")
}

pub struct Buffer {
    hb_buffer: *mut hb::hb_buffer_t,
}
#[allow(dead_code)]
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

    fn get_direction(&self) -> hb::hb_direction_t {
        unsafe { hb::hb_buffer_get_direction(self.hb_buffer) }
    }

    fn get_language(&self) -> &'static str {
        let lang = unsafe { hb::hb_buffer_get_language(self.hb_buffer) };
        language_to_string(lang)
    }

    fn guess_segment_properties(&mut self) {
        unsafe {
            hb::hb_buffer_guess_segment_properties(self.hb_buffer);
        }
    }

    fn get_segment_properties(&self) -> hb::hb_segment_properties_t {
        unsafe {
            let mut segment_props: hb::hb_segment_properties_t = Default::default();
            hb::hb_buffer_get_segment_properties(self.hb_buffer, &mut segment_props as *mut _);
            segment_props
        }
    }

    fn get_glyph_positions(&self) -> &mut [GlyphPosition] {
        unsafe {
            let mut length: u32 = 0;
            let glyph_pos = hb::hb_buffer_get_glyph_positions(self.hb_buffer,
                                                              &mut length as *mut u32);
            std::slice::from_raw_parts_mut(glyph_pos, length as usize)
        }
    }

    fn get_glyph_infos(&self) -> &mut [GlyphInfo] {
        unsafe {
            let mut length: u32 = 0;
            let glyph_infos = hb::hb_buffer_get_glyph_infos(self.hb_buffer,
                                                            &mut length as *mut u32);
            std::slice::from_raw_parts_mut(glyph_infos, length as usize)
        }
    }

    fn clear(&mut self) {
        unsafe {
            hb::hb_buffer_clear_contents(self.hb_buffer);
        }
    }
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("Buffer")
            .field("direction", &self.get_direction())
            .field("language", &self.get_language().to_owned())
            .finish()
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        let hb_buffer = unsafe { hb::hb_buffer_reference(self.hb_buffer) };
        Buffer { hb_buffer: hb_buffer }
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
             style: LayoutStyle)
             -> (&'a mut [GlyphPosition], &'a mut [GlyphInfo]) {
    assert!(buffer.get_direction() != hb::HB_DIRECTION_INVALID);
    unsafe {
        hb::hb_buffer_set_script(buffer.hb_buffer, HB_SCRIPT_MATH);
        hb::hb_buffer_set_language(buffer.hb_buffer, hb::hb_language_get_default());
        let mut features: Vec<hb::hb_feature_t> = Vec::with_capacity(2);
        if style.script_level >= 1 {
            let math_variants_tag = ot_tag!('s', 's', 't', 'y');
            let variant_num = style.script_level as u32;

            features.push(hb::hb_feature_t {
                tag: math_variants_tag,
                value: variant_num,
                start: 0,
                end: std::u32::MAX,
            })
        }
        features.push(hb::hb_feature_t {
            tag: ot_tag!('f', 'l', 'a', 'c'),
            value: 1,
            start: 0,
            end: std::u32::MAX,
        });
        hb::hb_shape(font.hb_font,
                     buffer.hb_buffer,
                     features.as_ptr(),
                     features.len() as u32);
    }
    let positions = buffer.get_glyph_positions();
    let infos = buffer.get_glyph_infos();
    (positions, infos)
}

fn shape_stretchy<'a>(font: &MathFont,
                      buffer: &'a Buffer,
                      horizontal: bool,
                      target_size: Position)
                      -> (&'a mut [GlyphPosition], &'a mut [GlyphInfo]) {
    unsafe {
        hb::hb_buffer_set_script(buffer.hb_buffer, HB_SCRIPT_MATH);
        hb::hb_buffer_set_language(buffer.hb_buffer, hb::hb_language_get_default());
        hb::hb_shape(font.hb_font,
                     buffer.hb_buffer,
                     std::ptr::null(),
                     0);
        hb::hb_ot_shape_math_stretchy(font.hb_font,
                                      buffer.hb_buffer,
                                      horizontal as i32,
                                      target_size);
    }
    let positions = buffer.get_glyph_positions();
    let infos = buffer.get_glyph_infos();
    (positions, infos)
}

pub fn box_from_glyph<T>(font: &MathFont, glyph: Glyph) -> MathBox<T> {
    let content = Content::Glyph(glyph);
    let mut bounds = font.get_glyph_bounds(glyph);
    bounds.extents.width = font.get_glyph_h_advance(glyph);

    assert_eq!(bounds.origin, Point { x: 0, y: 0 });

    let italic_correction = font.get_italic_correction(glyph);
    let mut logical_extents = bounds.extents;
    logical_extents.width += italic_correction;

    // if italic_correction == 0 {
    //     italic_correction = std::cmp::max(logical_extents.width - bounds.extents.width, 0);
    // }


    let mut top_accent_attachment = font.get_top_accent_attachment(glyph);
    top_accent_attachment = if top_accent_attachment == 0 {
        bounds.extents.width / 2
    } else {
        top_accent_attachment
    };

    MathBox {
        origin: bounds.origin,
        ink_extents: bounds.extents,
        logical_extents: logical_extents,
        italic_correction: italic_correction,
        top_accent_attachment: top_accent_attachment,
        content: content,
        ..Default::default()
    }
}


#[derive(Debug, Clone)]
pub struct MathShaper {
    buffer: RefCell<Buffer>,
}

impl MathShaper {
    pub fn new() -> MathShaper {
        MathShaper { buffer: RefCell::new(Buffer::new()) }
    }

    fn layout_boxes<T>(font: &MathFont,
                       style: LayoutStyle,
                       positions: &[GlyphPosition],
                       infos: &[GlyphInfo])
                       -> Vec<MathBox<T>> {
        let scale = font.scale_factor_for_script_level(style.script_level);
        let mut cursor = Point { x: 0, y: 0 };
        let list_iter = positions.iter().zip(infos.iter()).map(move |pos_info| {
            let pos = pos_info.0;
            let info = pos_info.1;
            let glyph = Glyph {
                glyph_code: info.codepoint,
                scale: PercentScale2D {
                    horiz: scale,
                    vert: scale,
                },
            };
            let mut new_box = box_from_glyph(font, glyph);

            let advance_x = pos.x_advance * scale;
            let advance_y = pos.y_advance * scale;
            new_box.origin.x += cursor.x + pos.x_offset * scale;
            new_box.origin.y += cursor.y - pos.y_offset * scale;
            // new_box.logical_extents.width = advance_width;
            cursor.x += advance_x;
            cursor.y -= advance_y;
            new_box
        });
        list_iter.collect()
    }

    pub fn shape<T>(&self, string: &str, font: &MathFont, style: LayoutStyle) -> Vec<MathBox<T>> {
        let mut buffer = self.buffer.borrow_mut();
        buffer.clear();
        buffer.set_direction(hb::HB_DIRECTION_LTR);
        buffer.add_str(string);
        buffer.guess_segment_properties();
        let (positions, infos) = shape(font, &buffer, style);
        MathShaper::layout_boxes(font, style, positions, infos)
    }

    pub fn shape_stretchy<T>(&self,
                             symbol: &str,
                             font: &MathFont,
                             horizontal: bool,
                             target_size: Position,
                             style: LayoutStyle)
                             -> Vec<MathBox<T>> {
        let mut buffer = self.buffer.borrow_mut();
        buffer.clear();
        buffer.set_direction(hb::HB_DIRECTION_LTR);
        buffer.add_str(symbol);
        buffer.guess_segment_properties();
        let (positions, infos) = shape_stretchy(font, &buffer, horizontal, target_size);
        MathShaper::layout_boxes(font, style, positions, infos)
    }
}

impl Default for MathShaper {
    fn default() -> MathShaper {
        MathShaper::new()
    }
}

#[cfg(test)]
mod tests {
    extern crate freetype;

    use std::cell::RefCell;

    use super::*;
    use super::shape;
    use super::super::font::MathFont;

    #[test]
    #[should_panic(expected = "already borrowed")]
    // tests that results of shaping cannot be used after mutating the buffer
    fn buffer_borrow_test() {
        let bytes = include_bytes!("../../tests/testfiles/latinmodern-math.otf");
        let ft_lib = freetype::Library::init().unwrap();
        let font = MathFont::from_bytes(bytes, 0, &ft_lib);
        let buffer = RefCell::new(Buffer::new());
        buffer.borrow_mut().set_direction(hb::HB_DIRECTION_LTR);
        buffer.borrow_mut().add_str("Test String");
        let borrowed_buffer = buffer.borrow();
        let (positions, _) = shape(&font, &borrowed_buffer, Default::default());

        buffer.borrow_mut().add_str("the borrow right before this literal should panic!");

        println!("{:?}", positions[0]);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_invalid_direction() {
        let bytes = include_bytes!("../../tests/testfiles/latinmodern-math.otf");
        let ft_lib = freetype::Library::init().unwrap();
        let font = MathFont::from_bytes(bytes, 0, &ft_lib);
        let mut buffer = Buffer::new();
        buffer.add_str("Test String");
        // This fails because no direction was set on the buffer.
        shape(&font, &buffer, Default::default());
    }

    #[test]
    fn stretchy_glyph_test() {
        let bytes = include_bytes!("../../tests/testfiles/latinmodern-math.otf");
        let ft_lib = freetype::Library::init().unwrap();
        let font = MathFont::from_bytes(bytes, 0, &ft_lib);

        let shaper = MathShaper::new();
        let shapy = shaper.shape_stretchy::<()>("√", &font, false, 4000, Default::default());
        println!("{:#?}", shapy);
        //panic!();
    }
}
