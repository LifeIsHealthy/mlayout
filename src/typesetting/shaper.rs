use std;
use std::iter::Iterator;
use std::cell::RefCell;

use types::{Glyph, LayoutStyle, PercentScale2D};
use super::math_box::{MathBox, Content, Point};
use super::font::{MathShaper, Position, GlyphPosition, GlyphInfo};

const HB_SCRIPT_MATH: u32 = ot_tag!('M', 'a', 't', 'h');

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
