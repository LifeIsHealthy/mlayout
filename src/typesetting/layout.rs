#![allow(unused_variables, dead_code, needless_lifetimes)]
extern crate freetype;

use types::*;
use std::iter;
use std::iter::IntoIterator;
use std::cmp::{max, min};
use std::fmt::Debug;

use super::font::{MathFont, hb};
use super::shaper::{MathShaper, box_from_glyph};
use super::math_box::{MathBox, Content, Extents, Point};
use super::multiscripts::*;

pub type ListIter<T> = Box<Iterator<Item = MathExpression<T>>>;
pub type BoxIter<T> = Box<Iterator<Item = MathBox<T>>>;

#[derive(Clone, Copy)]
pub struct LayoutOptions<'a, 'b: 'a> {
    pub font: &'a MathFont<'b>,
    pub shaper: &'a MathShaper,
    pub style: LayoutStyle,
    pub stretch_size: Option<StretchSize>,

    pub ft_library: &'a freetype::Library,
}

#[derive(Clone, Copy, Default)]
pub struct StretchSize {
    ascent: i32,
    descent: i32,
}

fn to_font_units(size: Length, default: i32, font: &MathFont) -> i32 {
    match size {
        Length::Em(val) => (font.ft_face.borrow().em_size() as f32 * val) as i32,
        Length::Points(val) => unimplemented!(),
        Length::Relative(val) => (val * default as f32) as i32,
    }
}

fn clamp<T: Ord, U: Into<Option<T>>>(value: T, min: U, max: U) -> T {
    if let Some(min) = min.into() {
        if value < min { return min };
    }
    if let Some(max) = max.into() {
        if value > max { return max };
    }
    value
}

fn calculate_stretch_size<T: Debug>(item: &Stretchable<T>, max_ascent: i32, max_descent: i32) -> StretchSize {
    if item.symmetric {
        max_ascent = max((max_ascent - axis), (max_descent + axis)) + axis;
        max_descent = max((max_ascent - axis), (max_descent + axis)) - axis;
    }

    let height = max_ascent + max_descent;
}

/// The trait that every Item in a math list satisfies so that the entire math list can be
/// layed out.
pub trait MathBoxLayout<'a, T: 'a> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a>;
    fn min_stretch_size<'b>(&self, options: LayoutOptions<'a, 'b>) -> Option<StretchSize> {
        None
    }
}

impl<'a, I, T: 'a + Debug> MathBoxLayout<'a, T> for I
    where I: 'a + IntoIterator<Item = MathExpression<T>>
{
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        let mut cursor = 0i32;
        let mut stretchables = Vec::new();

        let layouted = self.into_iter().enumerate().filter_map(move |(index, item)| {
            if let MathItem::Stretchy(stretchable) = item.content {
                stretchables.push((index, item));
                return None
            }
            let mut math_box: MathBox<T> = item.layout(options).collect();

            math_box.origin.x += cursor;
            cursor += math_box.logical_extents.width;
            Some(math_box)
        });

        if stretchables.is_empty() {
            Box::new(layouted)
        } else {
            let elements = layouted.collect::<Vec<_>>();
            if elements.is_empty() {
                unimplemented!()
            }
            let mut max_ascent = elements.iter().map(|elem| elem.ink_extents.ascent).max().unwrap();
            let mut max_descent = elements.iter().map(|elem| elem.ink_extents.descent).max().unwrap();

            let axis = options.font.get_math_constant(hb::HB_OT_MATH_CONSTANT_AXIS_HEIGHT);
            for (offset, (index, stretchable)) in stretchables.into_iter().enumerate() {
                let stretchy_elem = if let MathItem::Stretchy(stretchable) = stretchable.content {

                    let mut stretch_op = options;
                    stretch_op.stretch_size = Some(StretchSize { ascent: max_ascent, descent: max_descent });
                    stretchable.layout(stretch_op)
                } else {
                    unreachable!()
                };
                elements.insert(index + offset, stretchy_elem.collect());
            }

            Box::new(elements.into_iter())
        }
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Atom<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        if !self.top_right.is_empty() && !self.bottom_right.is_empty() {
            let mut subscript_options = options;
            subscript_options.style = options.style.subscript_style();
            let mut superscript_options = options;
            superscript_options.style = options.style.superscript_style();
            layout_sub_superscript(self.bottom_right.layout(subscript_options).collect(),
                                   self.top_right.layout(superscript_options).collect(),
                                   self.nucleus.layout(options).collect(),
                                   options.font,
                                   options.style)
        } else if !self.top_right.is_empty() {
            let mut superscript_options = options;
            superscript_options.style = options.style.superscript_style();
            layout_superscript(self.top_right.layout(superscript_options).collect(),
                               self.nucleus.layout(options).collect(),
                               options.font,
                               options.style)
        } else if !self.bottom_right.is_empty() {
            let mut subscript_options = options;
            subscript_options.style = options.style.subscript_style();
            layout_subscript(self.bottom_right.layout(subscript_options).collect(),
                             self.nucleus.layout(options).collect(),
                             options.font,
                             options.style)
        } else {
            self.nucleus.layout(options)
        }
    }
}


fn layout_superscript<'a, T: 'a>(mut superscript: MathBox<T>,
                                 nucleus: MathBox<T>,
                                 font: &MathFont,
                                 style: LayoutStyle)
                                 -> Box<Iterator<Item = MathBox<T>> + 'a> {
    let space_after_script = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SPACE_AFTER_SCRIPT);

    let superscript_shift_up = get_superscript_shift_up(&superscript, &nucleus, font, style);

    let superscript_kerning = get_attachment_kern(&nucleus,
                                                  &superscript,
                                                  CornerPosition::TopRight,
                                                  superscript_shift_up,
                                                  font);

    superscript.origin.x = nucleus.origin.x + nucleus.ink_extents.width + nucleus.italic_correction;
    superscript.origin.x += superscript_kerning;
    superscript.origin.y -= superscript_shift_up;
    superscript.logical_extents.width += space_after_script;
    let result = vec![nucleus, superscript];
    Box::new(result.into_iter())
}

fn layout_subscript<'a, T: 'a>(mut subscript: MathBox<T>,
                               nucleus: MathBox<T>,
                               font: &MathFont,
                               style: LayoutStyle)
                               -> Box<Iterator<Item = MathBox<T>> + 'a> {
    let space_after_script = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SPACE_AFTER_SCRIPT);

    let subscript_shift_dn = get_subscript_shift_dn(&subscript, &nucleus, font);

    let subscript_kerning = get_attachment_kern(&nucleus,
                                                &subscript,
                                                CornerPosition::BottomRight,
                                                subscript_shift_dn,
                                                font);

    subscript.origin.x = nucleus.origin.x + nucleus.ink_extents.width;
    subscript.origin.x += subscript_kerning;
    subscript.origin.y += subscript_shift_dn;
    subscript.logical_extents.width += space_after_script;
    let result = vec![nucleus, subscript];
    Box::new(result.into_iter())
}

fn layout_sub_superscript<'a, T: 'a>(mut subscript: MathBox<T>,
                                     mut superscript: MathBox<T>,
                                     nucleus: MathBox<T>,
                                     font: &MathFont,
                                     style: LayoutStyle)
                                     -> Box<Iterator<Item = MathBox<T>> + 'a> {
    let space_after_script = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SPACE_AFTER_SCRIPT);

    let (sub_shift, super_shift) =
        get_subsup_shifts(&subscript, &superscript, &nucleus, font, style);

    let subscript_kerning = get_attachment_kern(&nucleus,
                                                &subscript,
                                                CornerPosition::BottomRight,
                                                sub_shift,
                                                font);

    let superscript_kerning = get_attachment_kern(&nucleus,
                                                  &superscript,
                                                  CornerPosition::TopRight,
                                                  super_shift,
                                                  font);

    subscript.origin.x = nucleus.origin.x + nucleus.ink_extents.width;
    subscript.origin.x += subscript_kerning;
    subscript.origin.y += sub_shift;
    subscript.logical_extents.width += space_after_script;

    superscript.origin.x = nucleus.origin.x + nucleus.ink_extents.width + nucleus.italic_correction;
    superscript.origin.x += superscript_kerning;
    superscript.origin.y -= super_shift;
    superscript.logical_extents.width += space_after_script;

    let result = vec![nucleus, subscript, superscript];
    Box::new(result.into_iter())
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for OverUnder<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        let (has_over, has_under) = (!self.over.is_empty(), !self.under.is_empty());
        let nucleus = self.nucleus.layout(options);
        let nucleus = if has_over {
            let mut over_options = options;
            if !self.over_is_accent {
                over_options.style = over_options.style.superscript_style();
            }
            let font = &options.font;
            let style = options.style;
            let over = self.over.layout(over_options).collect();
            layout_over(over, nucleus.collect(), font, style, self.over_is_accent)
        } else {
            nucleus
        };

        if has_under {
            let mut under_options = options;
            if !self.under_is_accent {
                under_options.style = under_options.style.subscript_style();
            }
            let font = &options.font;
            let style = options.style;
            let under = self.under.layout(under_options).collect();
            layout_under(under, nucleus.collect(), font, style, self.under_is_accent)
        } else {
            nucleus
        }
    }
}

fn layout_over<'a, T: 'a>(mut over: MathBox<T>,
                          mut nucleus: MathBox<T>,
                          font: &'a MathFont,
                          style: LayoutStyle,
                          as_accent: bool)
                          -> Box<Iterator<Item = MathBox<T>> + 'a> {
    let over_gap = if as_accent {
        let accent_base_height = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_ACCENT_BASE_HEIGHT);
        if nucleus.ink_extents.ascent <= accent_base_height {
            accent_base_height - nucleus.ink_extents.ascent
        } else {
            -over.ink_extents.descent - accent_base_height
        }
    } else {
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_OVERBAR_VERTICAL_GAP)
    };
    let over_shift = over_gap + nucleus.ink_extents.ascent + over.ink_extents.descent;

    over.origin.y -= over_shift;

    // centering
    let center_difference = if as_accent {
        nucleus.top_accent_attachment + nucleus.origin.x - over.top_accent_attachment -
        over.origin.x
    } else {
        (nucleus.logical_extents.width - over.logical_extents.width) / 2
    };
    if center_difference < 0 {
        nucleus.origin.x -= center_difference;
    } else {
        over.origin.x += center_difference;
    }

    // over extra ascender
    let over_extra_ascender =
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_OVERBAR_EXTRA_ASCENDER);
    over.logical_extents.ascent += over_extra_ascender;

    // first the over then the nucleus to preserve the italic collection of the latter
    Box::new(vec![over, nucleus].into_iter())
}

fn layout_under<'a, T: 'a>(mut under: MathBox<T>,
                           mut nucleus: MathBox<T>,
                           font: &'a MathFont,
                           style: LayoutStyle,
                           as_accent: bool)
                           -> Box<Iterator<Item = MathBox<T>> + 'a> {
    let under_gap = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_UNDERBAR_VERTICAL_GAP);
    let under_shift = under_gap + nucleus.ink_extents.descent + under.ink_extents.ascent;
    under.origin.y += under_shift;

    // centering
    let width_difference = nucleus.ink_extents.width - under.ink_extents.width;
    if width_difference < 0 {
        nucleus.origin.x -= width_difference / 2;
    } else {
        under.origin.x += width_difference / 2;
    }

    // under extra ascender
    let under_extra_descender =
        font.get_math_constant(hb::HB_OT_MATH_CONSTANT_UNDERBAR_EXTRA_DESCENDER);
    under.logical_extents.descent += under_extra_descender;

    // first the under then the nucleus to preserve the italic collection of the latter
    Box::new(vec![under, nucleus].into_iter())
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for GeneralizedFraction<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        let denominator_options = LayoutOptions { style: options.style.cramped_style(), ..options };
        let mut numerator: MathBox<T> = self.numerator.layout(options).collect();
        let mut denominator: MathBox<T> = self.denominator.layout(denominator_options).collect();
        let font = &options.font;

        let axis_height = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_AXIS_HEIGHT);
        let default_thickness =
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_RULE_THICKNESS);

        let (numerator_shift_up, denominator_shift_dn) = if options.style.math_style ==
                                                            MathStyle::Inline {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_SHIFT_UP),
             font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_SHIFT_DOWN))
        } else {
            (font.get_math_constant(
                hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP
            ),
            font.get_math_constant(
                hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN)
             )
        };

        let (numerator_gap_min, denominator_gap_min) = if options.style.math_style ==
                                                          MathStyle::Inline {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_GAP_MIN),
             font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_GAP_MIN))
        } else {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUM_DISPLAY_STYLE_GAP_MIN),
             font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOM_DISPLAY_STYLE_GAP_MIN))
        };

        let numerator_shift_up = max(numerator_shift_up - axis_height,
                                     numerator_gap_min + default_thickness / 2 +
                                     numerator.ink_extents.descent);
        let denominator_shift_dn = max(denominator_shift_dn + axis_height,
                                       denominator_gap_min + default_thickness / 2 +
                                       denominator.ink_extents.ascent);

        numerator.origin.y -= axis_height;
        denominator.origin.y -= axis_height;

        numerator.origin.y -= numerator_shift_up;
        denominator.origin.y += denominator_shift_dn;

        // centering
        let width_difference = numerator.logical_extents.width - denominator.logical_extents.width;
        if width_difference < 0 {
            numerator.origin.x -= width_difference / 2;
        } else {
            denominator.origin.x += width_difference / 2;
        }

        let fraction_bar_extents = Extents {
            width: max(numerator.logical_extents.width,
                       denominator.logical_extents.width),
            ascent: default_thickness,
            descent: 0,
        };
        let fraction_bar = MathBox {
            origin: Point {
                x: min(numerator.origin.x, denominator.origin.x),
                y: -axis_height + default_thickness / 2,
            },
            ink_extents: fraction_bar_extents,
            logical_extents: fraction_bar_extents,
            content: Content::Filled,
            ..Default::default()
        };

        Box::new(vec![numerator, fraction_bar, denominator].into_iter())
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Root<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        let font = options.font;
        let line_thickness = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_RULE_THICKNESS);
        let vertical_gap = if options.style.math_style == MathStyle::Inline {
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_VERTICAL_GAP)
        } else {
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_DISPLAY_STYLE_VERTICAL_GAP)
        };
        let extra_ascender = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_EXTRA_ASCENDER);

        // calculate the needed surd height based on the height of the radicand
        let mut radicand: MathBox<T> = self.radicand.layout(options).collect();
        let needed_surd_height = radicand.ink_extents.height() + vertical_gap + line_thickness;

        // draw a stretched version of the surd
        let surd: Vec<MathBox<T>> = options.shaper
            .shape_stretchy("√", font, false, needed_surd_height, options.style);
        let mut surd: MathBox<T> = surd.into_iter().collect();

        // raise the surd so that its ascent is at least the radicand's ascender plus the radical
        // gap plus the line thickness of the radical rule
        let surd_excess_height = surd.ink_extents.height() -
                                 (radicand.ink_extents.height() + vertical_gap + line_thickness);

        surd.origin.y = (radicand.ink_extents.descent - surd.ink_extents.descent) +
                        surd_excess_height / 2;

        // place the radicand after the surd
        radicand.origin.x += surd.origin.x + surd.logical_extents.width;

        // the radical rule
        let radical_rule_extents = Extents {
            width: radicand.logical_extents.width,
            ascent: line_thickness,
            descent: 0,
        };
        let mut radical_rule: MathBox<T> = MathBox {
            origin: Point {
                x: surd.origin.x + surd.ink_extents.width,
                y: surd.origin.y - surd.ink_extents.ascent + line_thickness,
            },
            ink_extents: radical_rule_extents,
            logical_extents: radical_rule_extents,
            content: Content::Filled,
            ..Default::default()
        };

        let mut boxes = vec![];

        // typeset the root degree
        if !self.degree.is_empty() {
            let degree_bottom_raise_percent = PercentScale::new(font.get_math_constant(
                    hb::HB_OT_MATH_CONSTANT_RADICAL_DEGREE_BOTTOM_RAISE_PERCENT
            ) as u8);
            let kern_before =
                font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_KERN_BEFORE_DEGREE);
            let kern_after =
                font.get_math_constant(hb::HB_OT_MATH_CONSTANT_RADICAL_KERN_AFTER_DEGREE);
            let surd_height = surd.ink_extents.ascent + surd.ink_extents.descent;
            let degree_bottom = surd.origin.y + surd.ink_extents.descent -
                                surd_height * degree_bottom_raise_percent;

            let mut degree_options = options;
            degree_options.style.script_level += 2;
            degree_options.style.math_style = MathStyle::Inline;
            let mut degree = self.degree.layout(degree_options).collect::<MathBox<T>>();
            degree.origin.y += degree_bottom;
            degree.origin.x += kern_before;

            let surd_kern = kern_before + degree.logical_extents.width + kern_after;
            surd.origin.x += surd_kern;
            radicand.origin.x += surd_kern;
            radical_rule.origin.x += surd_kern;

            boxes.push(degree);
        }

        boxes.append(&mut vec![surd, radical_rule, radicand]);
        let mut combined_box = boxes.into_iter().collect::<MathBox<T>>();
        combined_box.logical_extents.ascent += extra_ascender;
        Box::new(iter::once(combined_box))
    }
}

impl<'a, T: 'a> MathBoxLayout<'a, T> for Field {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        match self {
            Field::Empty => Box::new(iter::empty::<MathBox<T>>()),
            Field::Glyph(glyph) => Box::new(iter::once(box_from_glyph(options.font, glyph))),
            Field::Unicode(content) => {
                let shaper = options.shaper;
                let shaping_result = shaper.shape(&content, options.font, options.style);
                Box::new(shaping_result.into_iter())
            }
        }
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Stretchable<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        let intrinsic_size = unimplemented!();
        let size = match (self.max_size, self.min_size) {
            (Some(max_size), Some(min_size)) => {}
            _ => {}
        };
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for MathItem<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        match self {
            MathItem::Field(field) => field.layout(options),
            MathItem::Space { width, ascent, descent } => {
                let extents = Extents {
                    width: to_font_units(width, 0, options.font),
                    ascent: to_font_units(ascent, 0, options.font),
                    descent: to_font_units(descent, 0, options.font),
                };
                let math_box = MathBox {
                    ink_extents: extents,
                    logical_extents: extents,
                    content: Content::Empty,
                    ..Default::default()
                };
                Box::new(iter::once(math_box))
            }
            MathItem::Atom(atom) => atom.layout(options),
            MathItem::GeneralizedFraction(frac) => frac.layout(options),
            MathItem::OverUnder(over_under) => over_under.layout(options),
            MathItem::List(list) => list.into_iter().layout(options),
            MathItem::Root(root) => root.layout(options),
            MathItem::Stretchy(stretchable) => stretchable.layout(options),
        }
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for MathExpression<T> {
    fn layout<'b>(self, options: LayoutOptions<'a, 'b>) -> Box<Iterator<Item = MathBox<T>> + 'a> {
        self.content.layout(options)
    }
}
