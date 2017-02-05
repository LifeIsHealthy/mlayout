
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
    pub glyph: Glyph,
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

    fn stretch_glyph<'a>(&'a self,
                         symbol: Glyph,
                         horizontal: bool,
                         target_size: u32)
                         -> Option<Box<Iterator<Item = ShapedGlyph> + 'a>>;

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
                    let origin = Point {
                        x: pos.x_offset,
                        y: pos.y_offset,
                    };
                    let advance = Point {
                        x: pos.x_advance,
                        y: pos.y_advance,
                    };
                    ShapedGlyph {
                        origin: origin * scale,
                        advance: advance * scale,
                        glyph: glyph,
                    }
                })
                .collect()
        };
        *self.buffer.borrow_mut() = Some(glyph_buffer.clear());
        let iterator = boxes.into_iter();
        Box::new(iterator)
    }

    fn glyph_bounds(&self, glyph: Glyph) -> Bounds {
        let glyph_extents =
            self.font.get_glyph_extents(glyph.glyph_code).unwrap_or(unsafe { std::mem::zeroed() });
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

fn point_with_offset(offset: i32, horizontal: bool) -> Point<i32> {
    if horizontal {
        Point { x: offset, y: 0 }
    } else {
        Point { x: 0, y: offset }
    }
}

fn try_base_glyph<'a>(shaper: &HarfbuzzShaper<'a>,
                      glyph: Glyph,
                      horizontal: bool,
                      target_size: u32)
                      -> Option<Box<Iterator<Item = ShapedGlyph>>> {
    let advance = if horizontal {
        shaper.font.get_glyph_h_advance(glyph.glyph_code) * glyph.scale.horiz
    } else {
        shaper.font.get_glyph_v_advance(glyph.glyph_code) * glyph.scale.vert
    };

    if advance >= target_size as i32 {
        let glyph = ShapedGlyph {
            glyph: glyph,
            origin: Default::default(),
            advance: point_with_offset(advance, horizontal),
        };
        Some(Box::new(std::iter::once(glyph)))
    } else {
        None
    }
}

struct VariantIterator<'a> {
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: Glyph,
    direction: hb::hb_direction_t,
    index: u32,
}

impl<'a> Iterator for VariantIterator<'a> {
    type Item = hb::hb_ot_math_glyph_variant_t;

    fn next(&mut self) -> Option<hb::hb_ot_math_glyph_variant_t> {
        let mut glyph_variant: hb::hb_ot_math_glyph_variant_t = unsafe { ::std::mem::zeroed() };
        let mut num_elements: u32 = 1;
        unsafe {
            hb::hb_ot_math_get_glyph_variants(self.shaper.font.as_raw(),
                                              self.glyph.glyph_code,
                                              self.direction,
                                              self.index,
                                              &mut num_elements,
                                              &mut glyph_variant)
        };
        self.index += 1;
        if num_elements == 1 {
            Some(glyph_variant)
        } else {
            None
        }
    }
}

fn try_variant<'a>(shaper: &HarfbuzzShaper<'a>,
                   glyph: Glyph,
                   horizontal: bool,
                   target_size: u32)
                   -> Option<Box<Iterator<Item = ShapedGlyph>>> {
    let direction = if horizontal {
        hb::HB_DIRECTION_LTR
    } else {
        hb::HB_DIRECTION_TTB
    };

    let mut iter = VariantIterator {
        shaper: shaper,
        glyph: glyph,
        direction: direction,
        index: 0,
    };

    iter.find(|glyph_variant| glyph_variant.advance >= target_size as i32)
        .map(move |glyph_variant| {
            let glyph = ShapedGlyph {
                glyph: Glyph { glyph_code: glyph_variant.glyph, ..glyph },
                origin: Default::default(),
                advance: point_with_offset(glyph_variant.advance, horizontal),
            };
            Box::new(std::iter::once(glyph)) as Box<Iterator<Item = _>>
        })
}

struct AssemblyIterator<'a> {
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: Glyph,
    direction: hb::hb_direction_t,
    index: u32,
}

impl<'a> Iterator for AssemblyIterator<'a> {
    type Item = hb::hb_ot_math_glyph_part_t;

    fn next(&mut self) -> Option<hb::hb_ot_math_glyph_part_t> {
        let mut glyph_part: hb::hb_ot_math_glyph_part_t = unsafe { ::std::mem::zeroed() };
        let mut num_elements: u32 = 1;
        let mut italics_correction: i32 = 0;
        unsafe {
            hb::hb_ot_math_get_glyph_assembly(self.shaper.font.as_raw(),
                                              self.glyph.glyph_code,
                                              self.direction,
                                              self.index,
                                              &mut num_elements,
                                              &mut glyph_part,
                                              &mut italics_correction)
        };
        self.index += 1;
        if num_elements == 1 {
            Some(glyph_part)
        } else {
            None
        }
    }
}

fn try_assembly<'a>(shaper: &'a HarfbuzzShaper<'a>,
                    glyph: Glyph,
                    horizontal: bool,
                    target_size: u32)
                    -> Option<Box<Iterator<Item = ShapedGlyph> + 'a>> {
    let direction = if horizontal {
        hb::HB_DIRECTION_LTR
    } else {
        hb::HB_DIRECTION_TTB
    };
    let min_connector_overlap: i32 = 0;

    let mut assembly_iter = AssemblyIterator {
        shaper: shaper,
        glyph: glyph,
        direction: direction,
        index: 0,
    };

    let mut full_advance_sum_non_ext: i32 = 0;
    let mut full_advance_sum_ext: i32 = 0;
    let mut part_count_non_ext: u32 = 0;
    let mut part_count_ext: u32 = 0;

    for part in assembly_iter.by_ref() {
        if horizontal {
            println!("part {:?}", part);
        }
        if part.flags == hb::HB_MATH_GLYPH_PART_FLAG_EXTENDER {
            full_advance_sum_ext += part.full_advance;
            part_count_ext += 1;
        } else {
            full_advance_sum_non_ext += part.full_advance;
            part_count_non_ext += 1;
        }
    }

    let a = full_advance_sum_non_ext - min_connector_overlap * (part_count_non_ext as i32 - 1);
    let b = full_advance_sum_ext - min_connector_overlap * part_count_ext as i32;
    if b == 0 {
        println!("b = {:?} for glyph: {:?}", b, glyph);
        return None;
    };
    let repeat_count_ext = ((target_size as i32 - a) as f32 / b as f32).ceil() as u32;

    let part_count = part_count_non_ext + part_count_ext * repeat_count_ext;

    if part_count == 0 || part_count > 2000 {
        println!("bad number of parts {:?}", part_count);
        return None;
    }

    let connector_overlap = if part_count >= 2 {
        // First determine the ideal overlap that would get closest to the target
        // size. The following quotient is integer operation and gives the best
        // lower approximation of the actual value with fractional pixels.
        let c = full_advance_sum_non_ext + repeat_count_ext as i32 * full_advance_sum_ext;
        let mut connector_overlap = (c - target_size as i32) / (part_count as i32 - 1);

        // We now consider the constraints on connectors. In general, only the
        // start of the first part and then end of the last part are not connected
        // so it is the minimum of StartConnector_i for all i > 0 and of
        // EndConnector_i for all i < glyphAssembly.part_record_count()-1. However,
        // if the first or last part is an extender then it will be connected too
        // with a copy of itself.
        //
        assembly_iter.index = 0;
        for (index, part) in assembly_iter.by_ref().enumerate() {
            let will_be_repeated = repeat_count_ext >= 2 &&
                                   part.flags == hb::HB_MATH_GLYPH_PART_FLAG_EXTENDER;
            if index < (part_count_ext + part_count_non_ext - 1) as usize || will_be_repeated {
                connector_overlap = ::std::cmp::min(connector_overlap, part.end_connector_length);
            }
            if index > 0 || will_be_repeated {
                connector_overlap = ::std::cmp::min(connector_overlap, part.start_connector_length);
            }
        }
        if connector_overlap < min_connector_overlap {
            println!("{:?} < {:?}", connector_overlap, min_connector_overlap);
            return None;
        };
        connector_overlap
    } else {
        0
    };

    assembly_iter.index = 0;
    let result = assembly_iter
        // Repeat the extenders `repeat_count_ext` times .
        .flat_map(move |part| {
            let repeat_count = if part.flags == hb::HB_MATH_GLYPH_PART_FLAG_EXTENDER {
                repeat_count_ext
            } else {
                1
            } as usize;
            ::std::iter::repeat(part).take(repeat_count)
        })
        // Offset the each glyph from the previous glyph by the advance of the part minus the
        // connector overlap.
        .scan(/* initial offset */ 0, move |current_offset, part| {
            let delta_offset = part.full_advance - connector_overlap;
            let origin = point_with_offset(*current_offset, horizontal);
            let glyph = ShapedGlyph {
                glyph: Glyph { glyph_code: part.glyph, ..glyph },
                origin: origin,
                advance: Point::default(),
            };
            *current_offset += delta_offset;
            Some((glyph, part.full_advance))
        })
        // Make sure that the last part's offset is added to the glyph offset.
        .enumerate()
        .map(move |(index, (glyph, advance))| if index >= (part_count - 1) as usize {
            ShapedGlyph { advance: point_with_offset(advance, horizontal), ..glyph }
        } else {
            glyph
        });

    Some(Box::new(result))
}

impl<'a> MathShaper for HarfbuzzShaper<'a> {
    fn math_constant(&self, c: MathConstant) -> i32 {
        unsafe { hb::hb_ot_math_get_constant(self.font.as_raw(), std::mem::transmute(c)) }
    }

    fn math_kerning(&self,
                    glyph: Glyph,
                    corner: CornerPosition,
                    correction_height: Position)
                    -> Position {
        let unscaled = unsafe {
            hb::hb_ot_math_get_glyph_kerning(self.font.as_raw(),
                                             glyph.glyph_code,
                                             std::mem::transmute(corner),
                                             correction_height / glyph.scale.vert)
        };
        unscaled * glyph.scale.horiz
    }

    fn italic_correction(&self, glyph: Glyph) -> Position {
        let unscaled = unsafe {
            hb::hb_ot_math_get_glyph_italics_correction(self.font.as_raw(), glyph.glyph_code)
        };
        unscaled * glyph.scale.horiz
    }

    fn top_accent_attachment(&self, glyph: Glyph) -> Position {
        let unscaled = unsafe {
            hb::hb_ot_math_get_glyph_top_accent_attachment(self.font.as_raw(), glyph.glyph_code)
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

    fn stretch_glyph<'b>(&'b self,
                         glyph: Glyph,
                         horizontal: bool,
                         target_size: u32)
                         -> Option<Box<Iterator<Item = ShapedGlyph> + 'b>> {
        try_base_glyph(self, glyph, horizontal, target_size)
            .or_else(|| try_variant(self, glyph, horizontal, target_size))
            .or_else(|| try_assembly(self, glyph, horizontal, target_size))
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
