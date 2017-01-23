
extern crate harfbuzz_rs;
extern crate harfbuzz_sys as hb;

use std;
use std::cell::RefCell;
use std::str::FromStr;

use types::{Glyph, CornerPosition, PercentScale, PercentScale2D, LayoutStyle};
use super::math_box::{Point, Extents, Bounds};

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShapedGlyph {
    pub origin: Point<i32>,
    pub advance: Point<i32>,
    pub glyph: Glyph
}

pub trait MathShaper {
    fn glyph_advance(&self, glyph: Glyph) -> Position;

    fn glyph_extents(&self, glyph: Glyph) -> (Position, Position);

    fn italic_correction(&self, glyph: Glyph) -> Position;

    fn top_accent_attachment(&self, glyph: Glyph) -> Position;

    fn math_kerning(&self,
                    glyph: Glyph,
                    corner: CornerPosition,
                    correction_height: Position)
                    -> Position;

    fn math_constant(&self, c: MathConstant) -> i32;

    fn shape_string(&self, string: &str, style: LayoutStyle) -> Box<Iterator<Item = ShapedGlyph>>;

    fn shape_stretchy(&self,
                      symbol: &str,
                      horizontal: bool,
                      target_size: u32,
                      style: LayoutStyle)
                      -> Box<Iterator<Item = ShapedGlyph>>;

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

    fn em_size(&self) -> Position;

    fn ppem(&self) -> (Position, Position) {
        (self.em_size(), self.em_size())
    }
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
        HarfbuzzShaper {
            font: font,
            buffer: buffer,
        }
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

    fn layout_boxes(&self,
                    style: LayoutStyle,
                    glyph_buffer: GlyphBuffer)
                    -> Box<Iterator<Item = ShapedGlyph>> {
        let boxes: Vec<ShapedGlyph> = {
            let positions = glyph_buffer.get_glyph_positions();
            let infos = glyph_buffer.get_glyph_infos();
            let scale = self.scale_factor_for_script_level(style.script_level);
            positions.iter()
                .zip(infos.iter())
                .map(move |(pos, info)| {
                    let glyph = Glyph {
                        glyph_code: info.codepoint,
                        scale: PercentScale2D {
                            horiz: scale,
                            vert: scale,
                        },
                    };
                    let origin = Point { x: pos.x_offset, y: pos.y_offset };
                    let advance = Point { x: pos.x_advance, y: pos.y_advance };
                    ShapedGlyph {
                        origin: origin * scale,
                        advance: advance * scale,
                        glyph: glyph
                    }
                })
                .collect()
        };
        *self.buffer.borrow_mut() = Some(glyph_buffer.clear());
        let iterator = boxes.into_iter();
        Box::new(iterator)
    }

    fn glyph_bounds(&self, glyph: Glyph) -> Bounds {
        let glyph_extents = self.font.get_glyph_extents(glyph.glyph_code).unwrap_or_default();
        let glyph_offset = self.font.get_glyph_h_origin(glyph.glyph_code).unwrap_or_default();
        let extents = Extents {
            width: glyph_extents.width,
            ascent: glyph_extents.y_bearing,
            descent: -(glyph_extents.height + glyph_extents.y_bearing),
        };
        let extents = extents * glyph.scale;
        let pos = Point {
            x: glyph_offset.0,
            y: glyph_offset.1,
        };
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

    fn shape_string(&self, string: &str, style: LayoutStyle) -> Box<Iterator<Item = ShapedGlyph>> {
        let glyph_buffer = self.shape_with_style(string, style);
        self.layout_boxes(style, glyph_buffer)
    }

    fn shape_stretchy(&self,
                      string: &str,
                      horizontal: bool,
                      target_size: u32,
                      style: LayoutStyle)
                      -> Box<Iterator<Item = ShapedGlyph>> {
        let glyph_buffer = self.shape_with_style(string, style);
        unsafe {
            hb::hb_ot_shape_math_stretchy(self.font.as_raw(),
                                          glyph_buffer.as_raw(),
                                          horizontal as i32,
                                          target_size as i32);
        }
        self.layout_boxes(style, glyph_buffer)
    }

    fn glyph_extents(&self, glyph: Glyph) -> (i32, i32) {
        let bounds = self.glyph_bounds(glyph);
        (bounds.extents.ascent, bounds.extents.descent)
    }

    fn glyph_advance(&self, glyph: Glyph) -> Position {
        self.font.get_glyph_h_advance(glyph.glyph_code) * glyph.scale.horiz
    }

    fn em_size(&self) -> Position {
        self.font.face().upem() as Position
    }
}
