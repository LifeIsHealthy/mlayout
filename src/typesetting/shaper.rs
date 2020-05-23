extern crate harfbuzz_rs;

use self::harfbuzz_rs::hb;
use std;
use std::cell::RefCell;
use std::cmp::min;

pub use self::harfbuzz_rs::Position;
use self::harfbuzz_rs::{
    shape, Blob, Feature, Font, GlyphBuffer, GlyphInfo, GlyphPosition, HarfbuzzObject, Shared, Tag,
    UnicodeBuffer,
};
use self::harfbuzz_rs::{FontFuncs, Glyph};
use super::math_box::{Drawable, Extents, MathBox, MathBoxContent, MathBoxMetrics, Vector};
use crate::types::{CornerPosition, LayoutStyle, PercentValue};

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

/// A structure that describes an individual glyph in a font.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MathGlyph {
    /// The font-specific glyph code
    pub glyph_code: u32,
    /// the utf-8 offset into the field that generated this glyph
    pub cluster: u32,
    /// The Offset at which the glyph should be rendered.
    pub offset: Vector<i32>,
    /// The glyph's advance width.
    pub advance_width: i32,
    /// The exact dimensions of the glyph's outline.
    pub extents: Extents<i32>,
    /// The italic correction to apply after this glyph.
    pub italic_correction: i32,
    /// The x-coordinate where a top accent should be attached.
    pub top_accent_attachment: i32,
}

impl MathBoxMetrics for MathGlyph {
    fn advance_width(&self) -> i32 {
        self.advance_width
    }

    fn extents(&self) -> Extents<i32> {
        self.extents
    }

    fn italic_correction(&self) -> i32 {
        self.italic_correction
    }

    fn top_accent_attachment(&self) -> i32 {
        self.top_accent_attachment
    }
}

pub trait MathShaper {
    /// Returns value of a constant for the current font.
    fn math_constant(&self, c: MathConstant) -> i32;

    fn shape(&self, string: &str, style: LayoutStyle, user_data: u64) -> MathBox;

    /// Returns a pointer to an OpenType-Math table.
    fn get_math_table(&self) -> &[u8];

    fn em_size(&self) -> Position;

    fn ppem(&self) -> (Position, Position) {
        (self.em_size(), self.em_size())
    }

    fn is_stretchable(&self, glyph: u32, horizontal: bool) -> bool;

    fn stretch_glyph(
        &self,
        glyph: u32,
        horizontal: bool,
        target_size: u32,
        style: LayoutStyle,
        user_data: u64,
    ) -> MathBox;

    fn math_kerning(
        &self,
        glyph: &MathGlyph,
        corner: CornerPosition,
        correction_height: Position,
    ) -> Position;
}

#[derive(Debug, Copy, Clone)]
pub struct HarfbuzzGlyph<'a> {
    pub origin: Vector<i32>,
    pub advance: Vector<i32>,
    pub glyph: u32,
    pub cluster: u32,
    shaper: &'a HarfbuzzShaper<'a>,
}

impl<'a> MathBoxMetrics for HarfbuzzGlyph<'a> {
    fn advance_width(&self) -> i32 {
        self.advance.x
    }

    fn extents(&self) -> Extents<i32> {
        let glyph_extents = self
            .shaper
            .font
            .get_glyph_extents(self.glyph)
            .unwrap_or(unsafe { std::mem::zeroed() });
        Extents {
            left_side_bearing: glyph_extents.x_bearing,
            width: glyph_extents.width,
            ascent: glyph_extents.y_bearing,
            descent: -(glyph_extents.height + glyph_extents.y_bearing),
        }
    }

    fn italic_correction(&self) -> i32 {
        unsafe {
            hb::hb_ot_math_get_glyph_italics_correction(self.shaper.font.as_raw(), self.glyph)
        }
    }

    fn top_accent_attachment(&self) -> i32 {
        unsafe {
            hb::hb_ot_math_get_glyph_top_accent_attachment(self.shaper.font.as_raw(), self.glyph)
        }
    }
}

impl<'a> HarfbuzzGlyph<'a> {
    fn origin(&self) -> Vector<i32> {
        let mut origin = self.origin;
        origin.y = -origin.y;
        origin
    }

    fn new(
        shaper: &'a HarfbuzzShaper<'a>,
        pos: GlyphPosition,
        info: GlyphInfo,
        _style: LayoutStyle,
    ) -> Self {
        let origin = Vector {
            x: pos.x_offset,
            y: pos.y_offset,
        };
        let advance = Vector {
            x: pos.x_advance,
            y: pos.y_advance,
        };
        HarfbuzzGlyph {
            shaper: shaper,
            origin: origin,
            advance: advance,
            glyph: info.codepoint,
            cluster: info.cluster,
        }
    }
}

impl<'a> From<HarfbuzzGlyph<'a>> for MathGlyph {
    fn from(hbglyph: HarfbuzzGlyph<'a>) -> MathGlyph {
        MathGlyph {
            glyph_code: hbglyph.glyph,
            cluster: hbglyph.cluster,
            offset: hbglyph.origin(),
            advance_width: hbglyph.advance_width(),
            extents: hbglyph.extents(),
            italic_correction: hbglyph.italic_correction(),
            top_accent_attachment: hbglyph.top_accent_attachment(),
        }
    }
}

/// The basic font structure used
#[derive(Debug)]
pub struct HarfbuzzShaper<'a> {
    pub font: Shared<Font<'a>>,
    pub no_cmap_font: Shared<Font<'a>>,
    buffer: RefCell<Option<UnicodeBuffer>>,
    math_table: Shared<Blob<'a>>,
}

pub struct IdentityFuncs;

impl FontFuncs for IdentityFuncs {
    fn get_nominal_glyph(&self, _font: &Font<'_>, unicode: char) -> Option<Glyph> {
        Some(unicode as Glyph)
    }
}

impl<'a> HarfbuzzShaper<'a> {
    pub fn new(font: Shared<Font>) -> HarfbuzzShaper {
        let buffer = Some(UnicodeBuffer::new()).into();
        let mut no_cmap_font = Font::create_sub_font(font.clone());
        no_cmap_font.set_font_funcs(IdentityFuncs);
        let math_table = font
            .face()
            .table_with_tag(b"MATH")
            .expect("MATH table must be present");
        HarfbuzzShaper {
            font,
            no_cmap_font: no_cmap_font.into(),
            buffer,
            math_table,
        }
    }

    // Return the font's scale factor for a given script level.
    fn scale_factor(&self, style: LayoutStyle) -> PercentValue {
        let percent = if style.script_level >= 1 {
            if style.script_level >= 2 {
                self.math_constant(MathConstant::ScriptScriptPercentScaleDown)
            } else {
                self.math_constant(MathConstant::ScriptPercentScaleDown)
            }
        } else {
            100
        };
        PercentValue::new(percent as u8)
    }

    fn shape_with_style(&self, string: &str, style: LayoutStyle, user_data: u64) -> MathBox {
        let mut buffer = self.buffer.borrow_mut().take().unwrap();

        buffer = buffer.add_str(string);
        *self.buffer.borrow_mut() = Some(buffer);
        self.do_shape(&self.font, style, user_data)
    }

    fn glyph_from_index(
        &self,
        glyph_index: u32,
        style: LayoutStyle,
        user_data: u64,
    ) -> Vec<MathGlyph> {
        let buffer = self.buffer.borrow_mut().take().unwrap();
        let buffer = buffer.add(glyph_index, 0);
        *self.buffer.borrow_mut() = Some(buffer);
        let math_box = self.do_shape(&self.no_cmap_font, style, user_data);
        match math_box.content {
            MathBoxContent::Drawable(Drawable::Glyphs { glyphs, .. }) => glyphs,
            _ => unreachable!(),
        }
    }

    fn do_shape(&self, font: &Font, style: LayoutStyle, user_data: u64) -> MathBox {
        let mut features = Vec::with_capacity(2);
        if style.script_level >= 1 {
            let math_variants_tag = Tag::new('s', 's', 't', 'y');
            let variant_num = style.script_level as u32;

            features.push(Feature::new(math_variants_tag, variant_num, ..));
        }
        if style.flat_accent {
            features.push(Feature::new(Tag::from(b"flac"), 1, ..));
        }

        let buffer = self
            .buffer
            .borrow_mut()
            .take()
            .expect("Buffer not available");
        let glyph_buffer = shape(font, buffer.set_script(Tag::from(b"Math")), &features);
        let math_box = {
            let shaped_glyphs = self.layout_boxes(&glyph_buffer, style);
            MathBox::with_glyphs(shaped_glyphs.collect(), self.scale_factor(style), user_data)
        };
        *self.buffer.borrow_mut() = Some(glyph_buffer.clear());

        math_box
    }

    fn layout_boxes<'b>(
        &'b self,
        glyph_buffer: &'b GlyphBuffer,
        style: LayoutStyle,
    ) -> impl 'b + Iterator<Item = MathGlyph> {
        let positions = glyph_buffer.get_glyph_positions();
        let infos = glyph_buffer.get_glyph_infos();
        positions.iter().zip(infos.iter()).map(move |(pos, info)| {
            let hb_glyph = HarfbuzzGlyph::new(self, *pos, *info, style);
            hb_glyph.into()
        })
    }
}

fn point_with_offset(offset: i32, horizontal: bool) -> Vector<i32> {
    if horizontal {
        Vector { x: offset, y: 0 }
    } else {
        Vector { x: 0, y: offset }
    }
}

impl<'a> MathShaper for HarfbuzzShaper<'a> {
    fn math_constant(&self, c: MathConstant) -> i32 {
        unsafe { hb::hb_ot_math_get_constant(self.font.as_raw(), c as _) }
    }

    fn get_math_table(&self) -> &[u8] {
        &self.math_table
    }

    fn shape(&self, string: &str, style: LayoutStyle, user_data: u64) -> MathBox {
        self.shape_with_style(string, style, user_data)
    }

    fn is_stretchable(&self, glyph: u32, horizontal: bool) -> bool {
        let direction = if horizontal {
            hb::HB_DIRECTION_LTR
        } else {
            hb::HB_DIRECTION_TTB
        };

        let variant_iter = VariantIterator {
            shaper: self,
            glyph: glyph,
            direction: direction,
            index: 0,
        };

        if variant_iter.len() > 0 {
            return true;
        }

        let assembly_iter = AssemblyIterator {
            shaper: self,
            glyph: glyph,
            direction: direction,
            index: 0,
        };

        if assembly_iter.len() > 0 {
            return true;
        }

        false
    }

    fn stretch_glyph(
        &self,
        glyph: u32,
        horizontal: bool,
        target_size: u32,
        style: LayoutStyle,
        user_data: u64,
    ) -> MathBox {
        // rescale target size for the current layout
        let target_size = target_size / self.scale_factor(style);

        let glyphs = try_base_glyph(self, glyph, horizontal, target_size, style, user_data)
            .or_else(|| try_variant(self, glyph, horizontal, target_size, style, user_data))
            .or_else(|| try_assembly(self, glyph, horizontal, target_size, style, user_data))
            .unwrap_or_else(|| {
                MathBox::with_glyphs(
                    self.glyph_from_index(glyph, style, user_data),
                    self.scale_factor(style),
                    user_data,
                )
            });

        // let result = {
        //     let glyph_indices = glyphs.iter().map(|shaped_glyph| shaped_glyph.glyph);
        //     let mut layout_style = LayoutStyle::new();
        //     layout_style.flat_accent = true;
        //     self.shape_glyph_indices(glyph_indices, LayoutStyle::new())
        // };
        // for (ref mut original_glyph, shaped_glyph) in glyphs.iter_mut().zip(result) {
        //     original_glyph.glyph = shaped_glyph.glyph;
        // }
        glyphs
    }

    fn em_size(&self) -> Position {
        self.font.face().upem() as Position
    }

    fn math_kerning(
        &self,
        glyph: &MathGlyph,
        corner: CornerPosition,
        correction_height: Position,
    ) -> Position {
        unsafe {
            hb::hb_ot_math_get_glyph_kerning(
                self.font.as_raw(),
                glyph.glyph_code,
                std::mem::transmute(corner),
                correction_height,
            )
        }
    }
}

fn try_base_glyph<'a>(
    shaper: &HarfbuzzShaper,
    glyph: u32,
    horizontal: bool,
    target_size: u32,
    style: LayoutStyle,
    user_data: u64,
) -> Option<MathBox> {
    let glyph = shaper.glyph_from_index(glyph, style, user_data)[0];

    let advance = if horizontal {
        glyph.extents.width
    } else {
        -glyph.extents.height()
    };

    if advance >= target_size as i32 {
        Some(MathBox::with_glyphs(
            vec![glyph],
            shaper.scale_factor(style),
            user_data,
        ))
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
            hb::hb_ot_math_get_glyph_variants(
                self.shaper.font.as_raw(),
                self.glyph,
                self.direction,
                self.index,
                &mut num_elements,
                &mut glyph_variant,
            )
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
            hb::hb_ot_math_get_glyph_variants(
                self.shaper.font.as_raw(),
                self.glyph,
                self.direction,
                self.index,
                &mut 0,
                std::ptr::null_mut(),
            )
        } as usize;
        let remaining_elements = total_variants - self.index as usize;
        (remaining_elements, Some(remaining_elements))
    }
}

impl<'a> ExactSizeIterator for VariantIterator<'a> {}

fn try_variant<'a>(
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: u32,
    horizontal: bool,
    target_size: u32,
    style: LayoutStyle,
    user_data: u64,
) -> Option<MathBox> {
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

    let variant = if style.as_accent {
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

    let glyphs = shaper.glyph_from_index(variant.glyph, style, user_data);
    Some(MathBox::with_glyphs(
        glyphs,
        shaper.scale_factor(style),
        user_data,
    ))
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
            hb::hb_ot_math_get_glyph_assembly(
                self.shaper.font.as_raw(),
                self.glyph,
                self.direction,
                self.index,
                &mut num_elements,
                &mut glyph_part,
                &mut italics_correction,
            )
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
            hb::hb_ot_math_get_glyph_assembly(
                self.shaper.font.as_raw(),
                self.glyph,
                self.direction,
                self.index,
                &mut 0,
                std::ptr::null_mut(),
                &mut 0,
            )
        } as usize;
        let remaining_elements = total_parts - self.index as usize;
        (remaining_elements, Some(remaining_elements))
    }
}

impl<'a> ExactSizeIterator for AssemblyIterator<'a> {}

fn try_assembly<'a>(
    shaper: &'a HarfbuzzShaper<'a>,
    glyph: u32,
    horizontal: bool,
    target_size: u32,
    style: LayoutStyle,
    user_data: u64,
) -> Option<MathBox> {
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
        if part.flags == hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER {
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
            let will_be_repeated =
                repeat_count_ext >= 2 && part.flags == hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER;
            if index < (part_count_ext + part_count_non_ext - 1) as usize || will_be_repeated {
                connector_overlap = min(connector_overlap, part.end_connector_length);
            }
            if index > 0 || will_be_repeated {
                connector_overlap = min(connector_overlap, part.start_connector_length);
            }
        }
        if connector_overlap < min_connector_overlap {
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
            let repeat_count = if part.flags == hb::HB_OT_MATH_GLYPH_PART_FLAG_EXTENDER {
                repeat_count_ext
            } else {
                1
            } as usize;
            std::iter::repeat(part).take(repeat_count)
        })
        // Offset the each glyph from the previous glyph by the advance of the part minus the
        // connector overlap.
        .scan(/* initial offset */ 0, move |current_offset, part| {
            let delta_offset = part.full_advance - connector_overlap;
            let origin = point_with_offset(*current_offset, horizontal);
            let glyphs = shaper.glyph_from_index(part.glyph, style, user_data);

            let mut math_box = MathBox::with_glyphs(glyphs, shaper.scale_factor(style), user_data);
            math_box.origin = origin;

            if horizontal {
                *current_offset += delta_offset;
            } else {
                *current_offset -= delta_offset;
            }
            Some(math_box)
        });

    Some(MathBox::with_vec(result.collect(), user_data))
}

#[cfg(test)]
mod test {

    #[test]
    fn test_assembly() {}
}
