extern crate harfbuzz_rs;
extern crate harfbuzz_sys as hb;

use std;
use std::cell::RefCell;
use std::str::FromStr;

use types::{CornerPosition, PercentValue, LayoutStyle};
use super::math_box::{Vector, Extents, MathBoxMetrics, MathBox};
pub use self::harfbuzz_rs::Position;
use self::harfbuzz_rs::{Font, FontFuncsBuilder, GlyphBuffer, Tag, Blob, HarfbuzzObject,
                        UnicodeBuffer};

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

pub trait MathGlyph: MathBoxMetrics {
    fn origin(&self) -> Vector<i32> {
        Vector::default()
    }

    fn math_kerning(&self, corner: CornerPosition, correction_height: Position) -> Position {
        0
    }
}

pub trait MathShaper<'a> {
    /// The  
    type Glyph: MathGlyph;

    /// Returns the unscaled value of the constant `c`.
    fn math_constant(&self, c: MathConstant) -> i32;

    fn shape(&self, string: &str, style: LayoutStyle) -> MathBox<'a, Self::Glyph>;

    /// Returns a pointer to an OpenType-Math table.
    fn get_math_table(&self) -> &[u8];

    fn em_size(&self) -> Position;

    fn ppem(&self) -> (Position, Position) {
        (self.em_size(), self.em_size())
    }
}


#[derive(Debug, Copy, Clone)]
pub struct HarfbuzzGlyph<'a> {
    pub origin: Vector<i32>,
    pub advance: Vector<i32>,
    pub scale: PercentValue,
    pub glyph: u32,
    pub cluster: u32,
    shaper: &'a HarfbuzzShaper<'a>,
}

impl<'a> MathBoxMetrics for HarfbuzzGlyph<'a> {
    fn advance_width(&self) -> i32 {
        self.advance.x * self.scale
    }

    fn extents(&self) -> Extents<i32> {
        let glyph_extents = self.shaper
            .font
            .get_glyph_extents(self.glyph)
            .unwrap_or(unsafe { std::mem::zeroed() });
        Extents {
            left_side_bearing: glyph_extents.x_bearing,
            width: glyph_extents.width,
            ascent: glyph_extents.y_bearing,
            descent: -(glyph_extents.height + glyph_extents.y_bearing),
        } * self.scale
    }

    fn italic_correction(&self) -> i32 {
        unsafe {
            hb::hb_ot_math_get_glyph_italics_correction(self.shaper.font.as_raw(), self.glyph) *
            self.scale
        }
    }

    fn top_accent_attachment(&self) -> i32 {
        unsafe {
            hb::hb_ot_math_get_glyph_top_accent_attachment(self.shaper.font.as_raw(), self.glyph) *
            self.scale
        }
    }
}

impl<'a> MathGlyph for HarfbuzzGlyph<'a> {
    fn origin(&self) -> Vector<i32> {
        let mut origin = self.origin;
        origin.y = -origin.y;
        origin * self.scale
    }

    fn math_kerning(&self, corner: CornerPosition, correction_height: Position) -> Position {
        unsafe {
            hb::hb_ot_math_get_glyph_kerning(self.shaper.font.as_raw(),
                                             self.glyph,
                                             std::mem::transmute(corner),
                                             correction_height / self.scale) *
            self.scale
        }
    }
}


/// The basic font structure used
#[derive(Debug, Clone)]
pub struct HarfbuzzShaper<'a> {
    pub font: Font<'a>,
    pub no_cmap_font: Font<'a>,
    buffer: RefCell<Option<UnicodeBuffer>>,
}

impl<'a> HarfbuzzShaper<'a> {
    pub fn new(font: Font) -> HarfbuzzShaper {
        let buffer = Some(UnicodeBuffer::new()).into();
        let mut no_cmap_font = font.create_sub_font();
        let mut ff_builder = FontFuncsBuilder::new();
        ff_builder.set_nominal_glyph_func(|_, _, chr| Some(chr as u32));
        let font_funcs = ff_builder.finish();
        no_cmap_font.set_font_funcs(&font_funcs, ());
        HarfbuzzShaper {
            font: font,
            no_cmap_font: no_cmap_font,
            buffer: buffer,
        }
    }

    fn scale_factor_for_script_level(&self, script_level: u8) -> PercentValue {
        let percent = if script_level >= 1 {
            if script_level >= 2 {
                self.math_constant(MathConstant::ScriptScriptPercentScaleDown)
            } else {
                self.math_constant(MathConstant::ScriptPercentScaleDown)
            }
        } else {
            100
        };
        PercentValue::new(percent as u8)
    }

    fn glyph_from_index(&self, glyph_index: u32) -> HarfbuzzGlyph {
        let (h_origin, v_origin) = self.font.get_glyph_h_origin(glyph_index).unwrap();
        let h_advance = self.font.get_glyph_h_advance(glyph_index);
        let v_advance = self.font.get_glyph_v_advance(glyph_index);
        HarfbuzzGlyph {
            shaper: self,
            glyph: glyph_index,
            origin: Vector {
                x: h_origin,
                y: v_origin,
            },
            advance: Vector {
                x: h_advance,
                y: v_advance,
            },
            scale: PercentValue::default(),
            cluster: 0,
        }
    }

    fn shape_with_style(&self, string: &str, style: LayoutStyle) -> MathBox<'a, HarfbuzzGlyph<'a>> {
        let mut buffer = self.buffer
            .borrow_mut()
            .take()
            .unwrap();

        println!("shape: {:?}", string);
        buffer = buffer.add_str(string);
        *self.buffer.borrow_mut() = Some(buffer);

        self.do_shape(&self.font, style)
    }

    fn shape_glyph_indices<I>(&self, indices: I, style: LayoutStyle) -> MathBox<'a, HarfbuzzGlyph<'a>>
        where I: Iterator<Item = u32>
    {
        let buffer = self.buffer
            .borrow_mut()
            .take()
            .unwrap();
        print!("shape glyphs: ");
        for (cluster, codepoint) in indices.enumerate() {
            print!("0x{:X} ", codepoint);
            unsafe {
                hb::hb_buffer_add(buffer.as_raw(), codepoint, cluster as u32);
            }
        }
        println!();
        unsafe {
            hb::hb_buffer_set_content_type(buffer.as_raw(), hb::HB_BUFFER_CONTENT_TYPE_UNICODE);
        }
        *self.buffer.borrow_mut() = Some(buffer);

        self.do_shape(&self.no_cmap_font, style)
    }

    fn do_shape(&self, font: &Font, style: LayoutStyle) -> MathBox<'a, HarfbuzzGlyph<'a>> {
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
        //if style.flat_accent {
        features.push(hb::hb_feature_t {
                          tag: Tag::new('f', 'l', 'a', 'c').0,
                          value: 1,
                          start: 0,
                          end: std::u32::MAX,
                      });
        //}

        let buffer = self.buffer
            .borrow_mut()
            .take()
            .unwrap();
        let glyph_buffer = buffer.set_script(Tag::from_str("Math").unwrap()).shape(font, &features);
        let shaped_glyphs = self.layout_boxes(&glyph_buffer, style);
        let math_box = self.math_box_from_glyphs(shaped_glyphs, style);

        *self.buffer.borrow_mut() = Some(glyph_buffer.clear());

        math_box
    }

    fn layout_boxes(&self, glyph_buffer: &GlyphBuffer, style: LayoutStyle) -> Vec<HarfbuzzGlyph> {
        let positions = glyph_buffer.get_glyph_positions();
        let infos = glyph_buffer.get_glyph_infos();
        positions.iter()
            .zip(infos.iter())
            .map(move |(pos, info)| {
                let origin = Vector {
                    x: pos.x_offset,
                    y: pos.y_offset,
                };
                let advance = Vector {
                    x: pos.x_advance,
                    y: pos.y_advance,
                };
                HarfbuzzGlyph {
                    origin: origin,
                    advance: advance,
                    glyph: info.codepoint,
                    scale: self.scale_factor_for_script_level(style.script_level),
                    shaper: self,
                    cluster: info.cluster,
                }
            })
            .collect()
    }

    fn math_box_from_glyphs<I>(&self, glyphs: I, style: LayoutStyle) -> MathBox<'a, HarfbuzzGlyph>
        where I: 'a + IntoIterator<Item = HarfbuzzGlyph<'a>>
    {
        let mut cursor = Vector { x: 0, y: 0 };
        let scale = self.scale_factor_for_script_level(style.script_level);
        let iterator = glyphs.into_iter().map(move |mut glyph| {
            glyph.scale = scale;
            let origin = glyph.origin();
            let advance = glyph.advance_width();
            let mut math_box = MathBox::with_glyph(glyph);
            math_box.origin = origin + cursor;
            cursor.x += advance;
            math_box
        });
        MathBox::with_iter(Box::new(iterator))
    }
}

fn point_with_offset(offset: i32, horizontal: bool) -> Vector<i32> {
    if horizontal {
        Vector { x: offset, y: 0 }
    } else {
        Vector { x: 0, y: offset }
    }
}

//fn get_single_glyph(shaper: &HarfbuzzShaper, glyph: u32, horizontal: bool) -> HarfbuzzGlyph {
//    let advance = if horizontal {
//        shaper.font.get_glyph_h_advance(glyph)
//    } else {
//        shaper.font.get_glyph_v_advance(glyph)
//    };
//
//    HarfbuzzGlyph {
//        glyph: glyph,
//        origin: Default::default(),
//        advance: point_with_offset(advance, horizontal),
//    }
//}

impl<'a> MathShaper<'a> for HarfbuzzShaper<'a> {
    type Glyph = HarfbuzzGlyph<'a>;

    fn math_constant(&self, c: MathConstant) -> i32 {
        unsafe { hb::hb_ot_math_get_constant(self.font.as_raw(), std::mem::transmute(c)) }
    }

    fn get_math_table(&self) -> &[u8] {
        let blob = unsafe {
            hb::hb_face_reference_table(self.font.face().as_raw(), Tag::from_str("MATH").unwrap().0)
        };
        let blob = unsafe { Blob::from_raw(blob) };
        blob.get_data()
    }

    fn shape(&self, string: &str, style: LayoutStyle) -> MathBox<'a, HarfbuzzGlyph<'a>> {
        self.shape_with_style(string, style)
    }

    //    fn is_stretchable(&self, glyph: u32, horizontal: bool) -> bool {
    //        let direction = if horizontal {
    //            hb::HB_DIRECTION_LTR
    //        } else {
    //            hb::HB_DIRECTION_TTB
    //        };
    //
    //        let variant_iter = VariantIterator {
    //            shaper: self,
    //            glyph: glyph,
    //            direction: direction,
    //            index: 0,
    //        };
    //
    //        if variant_iter.len() > 0 {
    //            return true;
    //        }
    //
    //        let assembly_iter = AssemblyIterator {
    //            shaper: self,
    //            glyph: glyph,
    //            direction: direction,
    //            index: 0,
    //        };
    //
    //        if assembly_iter.len() > 0 {
    //            return true;
    //        }
    //
    //        false
    //    }
    //
    //    fn stretch_glyph<'b>(&'b self,
    //                         glyph: u32,
    //                         horizontal: bool,
    //                         as_accent: bool,
    //                         target_size: u32)
    //                         -> Box<Iterator<Item = Self::Glyph> + 'b> {
    //        let base_glyph = get_single_glyph(self, glyph, horizontal);
    //
    //        let mut glyphs = try_base_glyph(base_glyph, horizontal, target_size)
    //            .or_else(|| try_variant(self, glyph, horizontal, as_accent, target_size))
    //            .or_else(|| try_assembly(self, glyph, horizontal, target_size))
    //            .map(|glyphs| glyphs.collect::<Vec<_>>())
    //            .unwrap_or_else(|| vec![base_glyph]);
    //
    //        let result = {
    //            let glyph_indices = glyphs.iter().map(|shaped_glyph| shaped_glyph.glyph);
    //            let mut layout_style = LayoutStyle::new();
    //            layout_style.flat_accent = true;
    //            self.shape_glyph_indices(glyph_indices, LayoutStyle::new())
    //        };
    //        for (ref mut original_glyph, shaped_glyph) in glyphs.iter_mut().zip(result) {
    //            original_glyph.glyph = shaped_glyph.glyph;
    //        }
    //        Box::new(glyphs.into_iter())
    //    }

    fn em_size(&self) -> Position {
        self.font.face().upem() as Position
    }
}

fn try_base_glyph<'a>(glyph: HarfbuzzGlyph<'a>,
                      horizontal: bool,
                      target_size: u32)
                      -> Option<Box<Iterator<Item = HarfbuzzGlyph<'a>> + 'a>> {
    let advance = if horizontal {
        glyph.advance.x
    } else {
        glyph.advance.y
    };

    if advance >= target_size as i32 {
        Some(Box::new(std::iter::once(glyph)))
    } else {
        None
    }
}

#[derive(Debug, Copy, Clone)]
struct VariantIterator<'a> {
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: u32,
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
                                              self.glyph,
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total_variants = unsafe {
            hb::hb_ot_math_get_glyph_variants(self.shaper.font.as_raw(),
                                              self.glyph,
                                              self.direction,
                                              self.index,
                                              &mut 0,
                                              std::ptr::null_mut())
        } as usize;
        let remaining_elements = total_variants - self.index as usize;
        (remaining_elements, Some(remaining_elements))
    }
}

impl<'a> ExactSizeIterator for VariantIterator<'a> {}

fn try_variant<'a>(shaper: &'a HarfbuzzShaper<'a>,
                   glyph: u32,
                   horizontal: bool,
                   as_accent: bool,
                   target_size: u32)
                   -> Option<Box<Iterator<Item = HarfbuzzGlyph<'a>> + 'a>> {
    let direction = if horizontal {
        hb::HB_DIRECTION_LTR
    } else {
        hb::HB_DIRECTION_TTB
    };

    let iter = VariantIterator {
        shaper: shaper,
        glyph: glyph,
        direction: direction,
        index: 0,
    };

    let variant =
        if as_accent {
            // return the largest variant that is smaller than the target size
            iter.filter(|&variant| variant.advance <= target_size as i32)
                .max_by_key(|&variant| variant.advance)
        } else {
            // return the smallest variant that is larger than the target size
            iter.filter(|&variant| variant.advance >= target_size as i32)
                .min_by_key(|&variant| variant.advance)
        };

    let variant = match variant {
        Some(variant) => variant,
        None => return None,
    };

    let glyph = HarfbuzzGlyph {
        glyph: variant.glyph,
        cluster: 0,
        origin: Default::default(),
        advance: point_with_offset(variant.advance, horizontal),
        scale: PercentValue::new(100),
        shaper: shaper,
    };
    Some(Box::new(std::iter::once(glyph)))
}

struct AssemblyIterator<'a> {
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: u32,
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
                                              self.glyph,
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total_parts = unsafe {
            hb::hb_ot_math_get_glyph_assembly(self.shaper.font.as_raw(),
                                              self.glyph,
                                              self.direction,
                                              self.index,
                                              &mut 0,
                                              std::ptr::null_mut(),
                                              &mut 0)
        } as usize;
        let remaining_elements = total_parts - self.index as usize;
        (remaining_elements, Some(remaining_elements))
    }
}

impl<'a> ExactSizeIterator for AssemblyIterator<'a> {}

fn try_assembly<'a>(shaper: &'a HarfbuzzShaper<'a>,
                    glyph: u32,
                    horizontal: bool,
                    target_size: u32)
                    -> Option<Box<Iterator<Item = HarfbuzzGlyph> + 'a>> {
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

    for part in &mut assembly_iter {
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
        // there probably is no glyph assembly for this glyph
        return None;
    };
    let repeat_count_ext = ((target_size as i32 - a) as f32 / b as f32).ceil() as u32;

    // Total number of parts needed to assemble the glyph including repetitions of extenders.
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
            let mut origin = point_with_offset(*current_offset, horizontal);
            let mut glyph = shaper.glyph_from_index(part.glyph);
            let extents = glyph.extents();
            if horizontal {
                origin.x -= extents.left_side_bearing;
            } else {
                origin.y += extents.descent;
            }
            glyph.origin = origin;
            glyph.advance = Vector::default();

            *current_offset += delta_offset;
            Some((glyph, part.full_advance))
        })
        // Make sure that the last part's advance is added to the assembly.
        .enumerate()
        .map(move |(index, (glyph, advance))| if index >= (part_count - 1) as usize {
            HarfbuzzGlyph { advance: point_with_offset(advance, horizontal), ..glyph }
        } else {
            glyph
        });

    Some(Box::new(result))
}
