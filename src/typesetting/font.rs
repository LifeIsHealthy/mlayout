
pub extern crate harfbuzz_rs;
pub extern crate harfbuzz_sys as hb;

use std;
use std::cell::RefCell;
use std::str::FromStr;

use types::{Glyph, GlyphCode, CornerPosition, PercentScale, PercentScale2D, LayoutStyle};
use super::math_box::{MathBox, Point, Extents, Bounds, Content};

pub use self::harfbuzz_rs::{Font, Position, GlyphPosition, GlyphExtents, GlyphInfo, GlyphBuffer,
                            Tag, Blob, HarfbuzzObject, UnicodeBuffer};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub enum MathConstant {
    ScriptPercentScaleDown = 0,
    ScriptScriptPercentScaleDown,
    DelimitedSubFormulaMinHeight,
    DisplayOperatorMinHeight,
    MathLeading,
    AxisHeight,
    AccentBaseHeight,
    FlattenedAccentBaseHeight,
    SubscriptShiftDown,
    SubscriptTopMax,
    SubscriptBaselineDropMin,
    SuperscriptShiftUp,
    SuperscriptShiftUpCramped,
    SuperscriptBottomMin,
    SuperscriptBaselineDropMax,
    SubSuperscriptGapMin,
    SuperscriptBottomMaxWithSubscript,
    SpaceAfterScript,
    UpperLimitGapMin,
    UpperLimitBaselineRiseMin,
    LowerLimitGapMin,
    LowerLimitBaselineDropMin,
    StackTopShiftUp,
    StackTopDisplayStyleShiftUp,
    StackBottomShiftDown,
    StackBottomDisplayStyleShiftDown,
    StackGapMin,
    StackDisplayStyleGapMin,
    StretchStackTopShiftUp,
    StretchStackBottomShiftDown,
    StretchStackGapAboveMin,
    StretchStackGapBelowMin,
    FractionNumeratorShiftUp,
    FractionNumeratorDisplayStyleShiftUp,
    FractionDenominatorShiftDown,
    FractionDenominatorDisplayStyleShiftDown,
    FractionNumeratorGapMin,
    FractionNumDisplayStyleGapMin,
    FractionRuleThickness,
    FractionDenominatorGapMin,
    FractionDenomDisplayStyleGapMin,
    SkewedFractionHorizontalGap,
    SkewedFractionVerticalGap,
    OverbarVerticalGap,
    OverbarRuleThickness,
    OverbarExtraAscender,
    UnderbarVerticalGap,
    UnderbarRuleThickness,
    UnderbarExtraDescender,
    RadicalVerticalGap,
    RadicalDisplayStyleVerticalGap,
    RadicalRuleThickness,
    RadicalExtraAscender,
    RadicalKernBeforeDegree,
    RadicalKernAfterDegree,
    RadicalDegreeBottomRaisePercent,
}

pub trait MathShaper: Clone {
    fn math_constant(&self, c: MathConstant) -> i32;

    fn italic_correction(&self, glyph: Glyph) -> Position;

    fn top_accent_attachment(&self, glyph: Glyph) -> Position;

    fn math_kerning(&self,
                    glyph: Glyph,
                    corner: CornerPosition,
                    correction_height: Position)
                    -> Position;

    fn glyph_box<T>(&self, glyph: Glyph) -> MathBox<T>;

    fn shape_string<T>(&self, string: &str, style: LayoutStyle) -> Vec<MathBox<T>>;

    fn shape_stretchy<T>(&self,
                         symbol: &str,
                         horizontal: bool,
                         target_size: u32,
                         style: LayoutStyle)
                         -> Vec<MathBox<T>>;

    fn get_math_table(&self) -> &[u8];

    fn scale_factor_for_script_level(&self, script_level: u8) -> PercentScale {
        let percent = if script_level >= 1 {
            if script_level >= 2 {
                self.math_constant(MathConstant::ScriptScriptPercentScaleDown)
            } else {
                self.math_constant(MathConstant::ScriptPercentScaleDown)
            }
        } else {
            100
        };
        PercentScale::new(percent as u8)
    }

    fn em_size(&self) -> u32;
}



/// The basic font structure used
#[derive(Debug, Clone)]
pub struct HarfbuzzShaper<'a> {
    pub font: Font<'a>,
    buffer: RefCell<Option<UnicodeBuffer>>,
}

impl<'a> HarfbuzzShaper<'a> {
    pub fn new(font: Font) -> HarfbuzzShaper {
        let buffer = Some(UnicodeBuffer::new()).into();
        HarfbuzzShaper { font: font, buffer: buffer }
    }

    fn shape_with_style(&self, string: &str, style: LayoutStyle) -> GlyphBuffer {
        let buffer = self.buffer.borrow_mut().take().unwrap();
        // hb::hb_buffer_set_language(buffer.hb_buffer, hb::hb_language_get_default());
        let mut features: Vec<hb::hb_feature_t> = Vec::with_capacity(2);
        if style.script_level >= 1 {
            let math_variants_tag = Tag::new('s', 's', 't', 'y');
            let variant_num = style.script_level as u32;

            features.push(hb::hb_feature_t {
                tag: math_variants_tag.0,
                value: variant_num,
                start: 0,
                end: std::u32::MAX,
            })
        }
        features.push(hb::hb_feature_t {
            tag: Tag::new('f', 'l', 'a', 'c').0,
            value: 1,
            start: 0,
            end: std::u32::MAX,
        });
        buffer.add_str(string)
            .set_script(Tag::from_str("Math").unwrap())
            .shape(&self.font, &features)
    }

    // pub fn shape_stretchy<T>(&self,
    //                          symbol: &str,
    //                          font: &MathFont,
    //                          horizontal: bool,
    //                          target_size: Position,
    //                          style: LayoutStyle)
    //                          -> Vec<MathBox<T>> {
    //     let mut buffer = self.buffer.borrow_mut();
    //     buffer.clear();
    //     buffer.set_direction(hb::HB_DIRECTION_LTR);
    //     buffer.add_str(symbol);
    //     buffer.guess_segment_properties();
    //     let (positions, infos) = shape_stretchy(font, &buffer, horizontal, target_size);
    //     MathShaper::layout_boxes(font, style, positions, infos)
    // }

    fn layout_boxes<T>(&self, style: LayoutStyle, glyph_buffer: GlyphBuffer) -> Vec<MathBox<T>> {
        let boxes = {
            let positions = glyph_buffer.get_glyph_positions();
            let infos = glyph_buffer.get_glyph_infos();
            let scale = self.scale_factor_for_script_level(style.script_level);
            let mut cursor = Point { x: 0, y: 0 };
            positions.iter()
                .zip(infos.iter())
                .map(move |pos_info| {
                    let pos = pos_info.0;
                    let info = pos_info.1;
                    let glyph = Glyph {
                        glyph_code: info.codepoint,
                        scale: PercentScale2D {
                            horiz: scale,
                            vert: scale,
                        },
                    };
                    let mut new_box = self.glyph_box(glyph);

                    let advance_x = pos.x_advance * scale;
                    let advance_y = pos.y_advance * scale;
                    new_box.origin.x += cursor.x + pos.x_offset * scale;
                    new_box.origin.y += cursor.y - pos.y_offset * scale;
                    // new_box.logical_extents.width = advance_width;
                    cursor.x += advance_x;
                    cursor.y -= advance_y;
                    new_box
                })
                .collect()
        };
        *self.buffer.borrow_mut() = Some(glyph_buffer.clear());
        boxes
    }

    fn glyph_bounds(&self, glyph: Glyph) -> Bounds {
        let glyph_extents = self.font.get_glyph_extents(glyph.glyph_code).unwrap_or_default();
        let glyph_offset = self.font.get_glyph_h_origin(glyph.glyph_code).unwrap_or_default();
        let extents = Extents {
            width: glyph_extents.width,
            ascent: glyph_extents.y_bearing,
            descent: - (glyph_extents.height + glyph_extents.y_bearing),
        };
        let extents = extents * glyph.scale;
        let pos = Point { x: glyph_offset.0, y: glyph_offset.1 };
        Bounds {
            extents: extents,
            origin: pos,
        }
    }
}

impl<'a> MathShaper for HarfbuzzShaper<'a> {
    fn math_constant(&self, c: MathConstant) -> i32 {
        unsafe { hb::hb_ot_layout_get_math_constant(self.font.as_raw(), c as u32) }
    }

    fn math_kerning(&self,
                    glyph: Glyph,
                    corner: CornerPosition,
                    correction_height: Position)
                    -> Position {
        let unscaled = unsafe {
            hb::hb_ot_layout_get_math_kerning(self.font.as_raw(),
                                              glyph.glyph_code,
                                              corner as hb::hb_ot_math_kern_t,
                                              correction_height / glyph.scale.vert)
        };
        unscaled * glyph.scale.horiz
    }

    fn italic_correction(&self, glyph: Glyph) -> Position {
        let unscaled = unsafe {
            hb::hb_ot_layout_get_math_italic_correction(self.font.as_raw(), glyph.glyph_code)
        };
        unscaled * glyph.scale.horiz
    }

    fn top_accent_attachment(&self, glyph: Glyph) -> Position {
        let unscaled = unsafe {
            hb::hb_ot_layout_get_math_top_accent_attachment(self.font.as_raw(), glyph.glyph_code)
        };
        unscaled * glyph.scale.horiz
    }

    fn get_math_table(&self) -> &[u8] {
        let blob = unsafe {
            hb::hb_face_reference_table(self.font.face().as_raw(), Tag::from_str("MATH").unwrap().0)
        };
        let blob = unsafe { Blob::from_raw(blob) };
        blob.get_data()
    }

    fn shape_string<T>(&self, string: &str, style: LayoutStyle) -> Vec<MathBox<T>> {
        let glyph_buffer = self.shape_with_style(string, style);
        self.layout_boxes(style, glyph_buffer)
    }

    fn shape_stretchy<T>(&self,
                         symbol: &str,
                         horizontal: bool,
                         target_size: u32,
                         style: LayoutStyle)
                         -> Vec<MathBox<T>> {
        Vec::new()
    }

    fn glyph_box<T>(&self, glyph: Glyph) -> MathBox<T> {
        let content = Content::Glyph(glyph);
        let mut bounds = self.glyph_bounds(glyph);
        bounds.extents.width = self.font.get_glyph_h_advance(glyph.glyph_code) * glyph.scale.horiz;

        assert_eq!(bounds.origin, Point { x: 0, y: 0 });

        let italic_correction = self.italic_correction(glyph);
        let mut logical_extents = bounds.extents;
        logical_extents.width += italic_correction;

        // if italic_correction == 0 {
        //     italic_correction = std::cmp::max(logical_extents.width - bounds.extents.width, 0);
        // }


        let mut top_accent_attachment = self.top_accent_attachment(glyph);
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

    fn em_size(&self) -> u32 {
        self.font.face().upem()
    }
}

// pub fn get_glyph_h_advance(&self, glyph: Glyph) -> i32 {
//     let unscaled = unsafe { hb::hb_font_get_glyph_h_advance(self.hb_font, glyph.glyph_code) };
//     unscaled * glyph.scale.horiz
// }
// pub fn get_glyph_v_advance(&self, glyph: Glyph) -> i32 {
//     let unscaled = unsafe { hb::hb_font_get_glyph_v_advance(self.hb_font, glyph.glyph_code) };
//     unscaled * glyph.scale.vert
// }
// pub fn get_glyph_bounds(&self, glyph: Glyph) -> Bounds {
//     let result = self.ft_face.borrow().load_glyph(glyph.glyph_code, face::NO_SCALE);
//     if result.is_err() {
//         let new_glyph_index = self.ft_face.borrow().get_char_index(0x221A);
//         println!("{:?}    {:?}", glyph.glyph_code, new_glyph_index);
//         self.ft_face
//             .borrow()
//             .load_glyph(new_glyph_index, face::NO_SCALE)
//             .expect("freetype could not load glyph");
//     }
//     let metrics = self.ft_face.borrow().glyph().metrics();
//     let extents = Extents {
//         width: metrics.width as i32,
//         ascent: metrics.horiBearingY as i32,
//         descent: metrics.height as i32 - metrics.horiBearingY as i32,
//     };
//     let extents = extents * glyph.scale;
//     let pos = Point { x: 0, y: 0 };
//     Bounds {
//         extents: extents,
//         origin: pos,
//     }
// }
