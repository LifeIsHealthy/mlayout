#![allow(unused_variables, dead_code)]
use types::*;
use std::iter::IntoIterator;
use std::cmp::{max, min};
use std::fmt::Debug;

use super::shaper::{MathShaper, MathConstant, ShapedGlyph};
use super::math_box::{MathBox, Extents, Point};
use super::multiscripts::*;
use super::stretchy::*;

#[derive(Copy, Clone)]
pub struct LayoutOptions<'a> {
    pub shaper: &'a MathShaper,
    pub style: LayoutStyle,
    pub stretch_size: Option<Extents<i32>>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct StretchProperties {
    pub intrinsic_size: u32,
    pub horizontal: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct OperatorProperties {
    pub stretch_properties: Option<StretchProperties>,
    pub leading_space: i32,
    pub trailing_space: i32,
}

impl Length {
    fn to_font_units(self, shaper: &MathShaper) -> i32 {
        if self.is_null() {
            return 0;
        }
        match self.unit {
            LengthUnit::Em => (shaper.em_size() as f32 * self.value) as i32,
            LengthUnit::Point => {
                Length::em(self.value / shaper.ppem().0 as f32).to_font_units(shaper)
            }
            LengthUnit::DisplayOperatorMinHeight => {
                (shaper.math_constant(MathConstant::DisplayOperatorMinHeight) as f32 *
                 self.value) as i32
            }
        }
    }
}


fn clamp<T: Ord, U: Into<Option<T>>>(value: T, min: U, max: U) -> T {
    if let Some(min) = min.into() {
        if value < min {
            return min;
        };
    }
    if let Some(max) = max.into() {
        if value > max {
            return max;
        };
    }
    value
}

fn math_box_from_shaped_glyphs<'a, T: 'a, I>(glyphs: I, shaper: &'a MathShaper) -> MathBox<'a, T>
    where I: 'a + IntoIterator<Item = ShapedGlyph>
{
    let mut cursor = Point { x: 0, y: 0 };
    let iterator = glyphs.into_iter().map(move |ShapedGlyph { mut origin, advance, glyph }| {
        let mut math_box = MathBox::with_glyph(glyph, shaper);
        origin.y = -origin.y;
        math_box.origin = origin + cursor;
        cursor.x += advance.x;
        cursor.y += advance.y;
        math_box
    });
    MathBox::with_iter(Box::new(iterator))
}

/// The trait that every Item in a math list satisfies so that the entire math list can be
/// layed out.
pub trait MathBoxLayout<'a, T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T>;
    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        None
    }
    fn can_stretch(&self, options: LayoutOptions<'a>) -> bool {
        self.operator_properties(options)
            .map(|operator_properties| operator_properties.stretch_properties.is_some())
            .unwrap_or_default()
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Field {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        match self {
            Field::Empty => MathBox::default(),
            Field::Glyph(glyph) => MathBox::with_glyph(glyph, options.shaper),
            Field::Unicode(content) => {
                let shaper = options.shaper;
                math_box_from_shaped_glyphs(shaper.shape_string(&content, options.style), shaper)
            }
        }
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Vec<MathExpression<T>> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        let boxes = layout_strechy_list(self, options);

        let mut cursor = 0i32;
        let mut previout_italic_correction = 0;
        let layouted = boxes.map(move |mut math_box| {
            if math_box.italic_correction() == 0 {
                cursor += previout_italic_correction;
            }
            math_box.origin.x += cursor;
            cursor += math_box.width();
            previout_italic_correction = math_box.italic_correction();
            math_box
        });
        MathBox::with_iter(Box::new(layouted))
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Atom<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        layout_sub_superscript(self.bottom_right, self.top_right, self.nucleus, options)
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        self.nucleus.operator_properties(options)
    }
}

fn layout_sub_superscript<'a, T: 'a + Debug>(subscript: MathExpression<T>,
                                             superscript: MathExpression<T>,
                                             nucleus: MathExpression<T>,
                                             options: LayoutOptions<'a>)
                                             -> MathBox<'a, T> {
    let mut subscript_options = options;
    subscript_options.style = options.style.subscript_style();
    let mut superscript_options = options;
    superscript_options.style = options.style.superscript_style();
    let subscript = subscript.into_option().map(|x| x.layout(subscript_options));
    let superscript = superscript.into_option().map(|x| x.layout(superscript_options));
    let nucleus_is_largeop = match nucleus.content {
        MathItem::Operator(Operator { is_large_op, .. }) => is_large_op,
        _ => false,
    };
    let mut nucleus = nucleus.layout(options);

    let space_after_script = options.shaper.math_constant(MathConstant::SpaceAfterScript);

    if subscript.is_none() && superscript.is_none() {
        return nucleus;
    }

    let mut result = Vec::with_capacity(4);
    match (subscript, superscript) {
        (Some(mut subscript), Some(mut superscript)) => {
            let (sub_shift, super_shift) = get_subsup_shifts(&subscript,
                                                             &superscript,
                                                             &nucleus,
                                                             options.shaper,
                                                             options.style);
            position_attachment(&mut subscript,
                                &mut nucleus,
                                nucleus_is_largeop,
                                CornerPosition::BottomRight,
                                sub_shift,
                                options.shaper);
            position_attachment(&mut superscript,
                                &mut nucleus,
                                nucleus_is_largeop,
                                CornerPosition::TopRight,
                                super_shift,
                                options.shaper);
            result.push(nucleus);
            result.push(subscript);
            result.push(superscript);
        }
        (Some(mut subscript), None) => {
            let sub_shift = get_subscript_shift_dn(&subscript, &nucleus, options.shaper);
            position_attachment(&mut subscript,
                                &mut nucleus,
                                nucleus_is_largeop,
                                CornerPosition::BottomRight,
                                sub_shift,
                                options.shaper);
            result.push(nucleus);
            result.push(subscript);
        }
        (None, Some(mut superscript)) => {
            let super_shift =
                get_superscript_shift_up(&superscript, &nucleus, options.shaper, options.style);
            position_attachment(&mut superscript,
                                &mut nucleus,
                                nucleus_is_largeop,
                                CornerPosition::TopRight,
                                super_shift,
                                options.shaper);
            result.push(nucleus);
            result.push(superscript);
        }
        (None, None) => unreachable!(),
    }

    let mut space = MathBox::empty(Extents::new(space_after_script, None, None));
    space.origin.x = result.iter()
        .map(|math_box| math_box.origin.x + math_box.width())
        .max()
        .unwrap_or_default();
    result.push(space);

    MathBox::with_vec(result)
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for OverUnder<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        if self.is_limits && options.style.math_style == MathStyle::Inline {
            return layout_sub_superscript(self.under, self.over, self.nucleus, options);
        }
        let nucleus_is_largeop = match self.nucleus.content {
            MathItem::Operator(Operator { is_large_op, .. }) => is_large_op,
            _ => false,
        };

        let mut over_options = LayoutOptions { style: options.style.inline_style(), ..options };
        if !self.over_is_accent {
            over_options.style = over_options.style.superscript_style();
        }
        let mut under_options = LayoutOptions { style: options.style.inline_style(), ..options };
        if !self.under_is_accent {
            under_options.style = under_options.style.subscript_style();
        }
        let mut expressions = [(self.nucleus.into_option(), options),
                               (self.over.into_option(), over_options),
                               (self.under.into_option(), under_options)];
        let mut boxes = [None, None, None];

        for (index, &mut (ref mut expr, options)) in expressions.iter_mut().enumerate() {
            // first take and layout non-stretchy subexpressions
            if !expr.as_ref().map(|x| x.can_stretch(options)).unwrap_or_default() {
                boxes[index] = expr.take().map(|expr| expr.layout(options));
            } else {
                println!("This can stretch: {:?}", expr);
            }
        }
        // get the maximal width of the non-stretchy items
        let max_width = boxes.iter()
            .map(|math_box| math_box.as_ref().map(|x| x.width()).unwrap_or_default())
            .max()
            .unwrap_or_default();

        // layout the rest
        for (index, &mut (ref mut expr, mut options)) in expressions.iter_mut().enumerate() {
            options.stretch_size = options.stretch_size
                .map(|size| Extents { width: max_width, ..size })
                .or(Some(Extents { width: max_width, ..Default::default() }));
            if let Some(stretched_box) = expr.take().map(|expr| expr.layout(options)) {
                boxes[index] = Some(stretched_box);
            }
        }

        let nucleus = boxes[0].take().unwrap_or_default();
        let nucleus = if let Some(over) = boxes[1].take() {
            let (_, LayoutOptions { style, shaper, .. }) = expressions[1];
            layout_over(over,
                        nucleus,
                        shaper,
                        style,
                        self.over_is_accent,
                        nucleus_is_largeop)
        } else {
            nucleus
        };

        if let Some(under) = boxes[2].take() {
            let (_, LayoutOptions { style, shaper, .. }) = expressions[2];
            layout_under(under,
                         nucleus,
                         shaper,
                         style,
                         self.under_is_accent,
                         nucleus_is_largeop)
        } else {
            nucleus
        }
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        self.nucleus.operator_properties(options)
    }
}

fn layout_over<'a, T: 'a>(mut over: MathBox<'a, T>,
                          mut nucleus: MathBox<'a, T>,
                          shaper: &MathShaper,
                          style: LayoutStyle,
                          as_accent: bool,
                          nucleus_is_large_op: bool)
                          -> MathBox<'a, T> {
    let over_gap = if as_accent {
        let accent_base_height = shaper.math_constant(MathConstant::AccentBaseHeight);
        if nucleus.ascent() <= accent_base_height {
            accent_base_height - nucleus.ascent()
        } else {
            -over.descent() - accent_base_height
        }
    } else {
        shaper.math_constant(MathConstant::OverbarVerticalGap)
    };
    let over_shift = over_gap + nucleus.ascent() + over.descent();

    over.origin.y -= over_shift;

    // centering
    let center_difference = if as_accent {
        nucleus.top_accent_attachment() + nucleus.origin.x - over.top_accent_attachment() -
        over.origin.x
    } else {
        (nucleus.width() - over.width()) / 2
    };
    if center_difference < 0 {
        nucleus.origin.x -= center_difference;
    } else {
        over.origin.x += center_difference;
    }

    // LargeOp italic correction
    if nucleus_is_large_op {
        over.origin.x += nucleus.italic_correction() / 2;
    }

    // over extra ascender
    let over_extra_ascender = shaper.math_constant(MathConstant::OverbarExtraAscender);
    // over.logical_extents.ascent += over_extra_ascender;

    // first the over then the nucleus to preserve the italic collection of the latter
    MathBox::with_vec(vec![over, nucleus])
}

fn layout_under<'a, T: 'a>(mut under: MathBox<'a, T>,
                           mut nucleus: MathBox<'a, T>,
                           shaper: &MathShaper,
                           style: LayoutStyle,
                           as_accent: bool,
                           nucleus_is_large_op: bool)
                           -> MathBox<'a, T> {
    let under_gap = shaper.math_constant(MathConstant::UnderbarVerticalGap);
    let under_shift = under_gap + nucleus.descent() + under.ascent();
    under.origin.y += under_shift;

    // centering
    let width_difference = nucleus.width() - under.width();
    if width_difference < 0 {
        nucleus.origin.x -= width_difference / 2;
    } else {
        under.origin.x += width_difference / 2;
    }

    // LargeOp italic correction
    if nucleus_is_large_op {
        under.origin.x -= nucleus.italic_correction() / 2;
    }

    // under extra descender
    let under_extra_descender = shaper.math_constant(MathConstant::UnderbarExtraDescender);
    // under.logical_extents.descent += under_extra_descender;

    // first the under then the nucleus to preserve the italic collection of the latter
    MathBox::with_vec(vec![under, nucleus])
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for GeneralizedFraction<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        let mut numerator_options = options;
        if options.style.math_style == MathStyle::Display {
            numerator_options.style.math_style = MathStyle::Inline;
        } else {
            numerator_options.style.script_level += 1;
        }
        let denominator_options =
            LayoutOptions { style: numerator_options.style.cramped_style(), ..options };
        let mut numerator = self.numerator.layout(numerator_options);
        let mut denominator = self.denominator.layout(denominator_options);
        let shaper = &options.shaper;

        let axis_height = shaper.math_constant(MathConstant::AxisHeight);
        let default_thickness = shaper.math_constant(MathConstant::FractionRuleThickness);

        let (numerator_shift_up, denominator_shift_dn) = if options.style.math_style ==
                                                            MathStyle::Inline {
            (shaper.math_constant(MathConstant::FractionNumeratorShiftUp),
             shaper.math_constant(MathConstant::FractionDenominatorShiftDown))
        } else {
            (shaper.math_constant(MathConstant::FractionNumeratorDisplayStyleShiftUp),
             shaper.math_constant(MathConstant::FractionDenominatorDisplayStyleShiftDown))
        };

        let (numerator_gap_min, denominator_gap_min) = if options.style.math_style ==
                                                          MathStyle::Inline {
            (shaper.math_constant(MathConstant::FractionNumeratorGapMin),
             shaper.math_constant(MathConstant::FractionDenominatorGapMin))
        } else {
            (shaper.math_constant(MathConstant::FractionNumDisplayStyleGapMin),
             shaper.math_constant(MathConstant::FractionDenomDisplayStyleGapMin))
        };

        let numerator_shift_up = max(numerator_shift_up - axis_height,
                                     numerator_gap_min + default_thickness / 2 +
                                     numerator.descent());
        let denominator_shift_dn = max(denominator_shift_dn + axis_height,
                                       denominator_gap_min + default_thickness / 2 +
                                       denominator.ascent());

        numerator.origin.y -= axis_height;
        denominator.origin.y -= axis_height;

        numerator.origin.y -= numerator_shift_up;
        denominator.origin.y += denominator_shift_dn;

        // centering
        let width_difference = numerator.width() - denominator.width();
        if width_difference < 0 {
            numerator.origin.x -= width_difference / 2;
        } else {
            denominator.origin.x += width_difference / 2;
        }

        // the fraction rule
        let origin = Point {
            x: min(numerator.origin.x, denominator.origin.x),
            y: -axis_height,
        };
        let target = Point { x: origin.x + max(numerator.width(), denominator.width()), ..origin };
        let fraction_rule = MathBox::with_line(origin, target, default_thickness as u32);

        MathBox::with_vec(vec![numerator, fraction_rule, denominator])
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        self.numerator.operator_properties(options)
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Root<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        let shaper = options.shaper;
        let line_thickness = shaper.math_constant(MathConstant::RadicalRuleThickness);
        let vertical_gap = if options.style.math_style == MathStyle::Inline {
            shaper.math_constant(MathConstant::RadicalVerticalGap)
        } else {
            shaper.math_constant(MathConstant::RadicalDisplayStyleVerticalGap)
        };
        let extra_ascender = shaper.math_constant(MathConstant::RadicalExtraAscender);

        // calculate the needed surd height based on the height of the radicand
        let mut radicand = self.radicand.layout(options);
        let needed_surd_height = (radicand.height() + vertical_gap + line_thickness) as u32;

        // draw a stretched version of the surd
        let mut surd = options.shaper.shape_string("√", options.style);
        let surd = match surd.next() {
            Some(shaped_glyph) => {
                options.shaper
                    .stretch_glyph(shaped_glyph.glyph, false, needed_surd_height)
                    .expect("could not stretch surd")
            }
            None => Box::new(::std::iter::empty()),
        };
        let mut surd = math_box_from_shaped_glyphs(surd, options.shaper);

        // raise the surd so that its ascent is at least the radicand's ascender plus the radical
        // gap plus the line thickness of the radical rule
        let surd_excess_height = surd.height() -
                                 (radicand.height() + vertical_gap + line_thickness);

        surd.origin.y = (radicand.descent() - surd.descent()) + surd_excess_height / 2;

        // place the radicand after the surd
        radicand.origin.x += surd.origin.x + surd.width();

        // the radical rule
        let origin = Point {
            x: surd.origin.x + surd.width(),
            y: surd.origin.y - surd.ascent() + line_thickness / 2,
        };
        let target = Point { x: origin.x + radicand.width(), ..origin };
        let mut radical_rule = MathBox::with_line(origin, target, line_thickness as u32);

        let mut boxes = vec![];

        // typeset the self degree
        if !self.degree.is_empty() {
            let degree_bottom_raise_percent = PercentScale::new(shaper.math_constant(
                    MathConstant::RadicalDegreeBottomRaisePercent
            ) as u8);
            let kern_before = shaper.math_constant(MathConstant::RadicalKernBeforeDegree);
            let kern_after = shaper.math_constant(MathConstant::RadicalKernAfterDegree);
            let surd_height = surd.ascent() + surd.descent();
            let degree_bottom = surd.origin.y + surd.descent() -
                                surd_height * degree_bottom_raise_percent;

            let mut degree_options = options;
            degree_options.style.script_level += 2;
            degree_options.style.math_style = MathStyle::Inline;
            let mut degree = self.degree.layout(degree_options);
            degree.origin.y += degree_bottom;
            degree.origin.x += kern_before;

            let surd_kern = kern_before + degree.width() + kern_after;
            surd.origin.x += surd_kern;
            radicand.origin.x += surd_kern;
            radical_rule.origin.x += surd_kern;

            boxes.push(degree);
        }

        boxes.append(&mut vec![surd, radical_rule, radicand]);
        MathBox::with_vec(boxes)
        // TODO
        // let mut combined_box = boxes.into_iter().collect::<MathBox>();
        // combined_box.logical_extents.ascent += extra_ascender;
        // Box::new(iter::once(combined_box))
    }
}

impl Operator {
    fn layout_stretchy<'a, T: 'a + Debug>(self,
                                          needed_height: u32,
                                          needed_width: u32,
                                          options: LayoutOptions<'a>)
                                          -> MathBox<'a, T> {
        match self.field {
            Field::Unicode(ref string) => {
                let mut shape_result = options.shaper.shape_string(string, options.style);
                let first_glyph = shape_result.next();
                if needed_height > 0 {
                    let stretched = first_glyph.and_then(move |shaped_glyph| {
                        options.shaper.stretch_glyph(shaped_glyph.glyph, false, needed_height)
                    });
                    if let Some(stretched) = stretched {
                        let mut math_box = math_box_from_shaped_glyphs(stretched, options.shaper);

                        if let Some(stretch_constraints) = self.stretch_constraints {
                            if stretch_constraints.symmetric {
                                let axis_height = options.shaper
                                    .math_constant(MathConstant::AxisHeight);
                                let shift_up = (math_box.descent() - math_box.ascent()) / 2 +
                                               axis_height;
                                math_box.origin.y -= shift_up;
                            } else {
                                let stretch_size = options.stretch_size.unwrap_or_default();
                                let excess_ascent = math_box.ascent() - stretch_size.ascent;
                                let excess_descent = math_box.descent() - stretch_size.descent;
                                math_box.origin.y += (excess_ascent - excess_descent) / 2;
                            }
                        }

                        return math_box;
                    }
                }
                if needed_width > 0 {
                    let stretched = first_glyph.and_then(move |shaped_glyph| {
                        options.shaper.stretch_glyph(shaped_glyph.glyph, true, needed_width)
                    });
                    if let Some(stretched) = stretched {
                        return math_box_from_shaped_glyphs(stretched, options.shaper);
                    }
                }

                math_box_from_shaped_glyphs(first_glyph, options.shaper)
            }
            _ => unimplemented!(),
        }
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for Operator {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        match (options.stretch_size, self.stretch_constraints) {
            (Some(stretch_size), Some(stretch_constraints)) => {
                let min_size = stretch_constraints.min_size
                    .map(|size| size.to_font_units(options.shaper));
                let max_size = stretch_constraints.max_size
                    .map(|size| size.to_font_units(options.shaper));
                let mut needed_height = if stretch_constraints.symmetric {
                    let axis_height = options.shaper.math_constant(MathConstant::AxisHeight);
                    max(stretch_size.ascent - axis_height,
                        axis_height + stretch_size.descent) * 2
                } else {
                    (stretch_size.ascent + stretch_size.descent)
                };
                needed_height = clamp(needed_height, min_size, max_size);
                let needed_height = max(0, needed_height) as u32;
                self.layout_stretchy(needed_height, stretch_size.width as u32, options)
            }
            _ => {
                if self.is_large_op && options.style.math_style == MathStyle::Display {
                    let display_min_height = options.shaper
                        .math_constant(MathConstant::DisplayOperatorMinHeight);
                    self.layout_stretchy(display_min_height as u32, 0, options)
                } else {
                    self.field.layout(options)
                }
            }
        }
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        Some(OperatorProperties {
            stretch_properties: self.stretch_constraints.as_ref().map(|_| Default::default()),
            leading_space: self.leading_space.to_font_units(options.shaper),
            trailing_space: self.trailing_space.to_font_units(options.shaper),
        })
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for MathSpace {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        let extents = Extents {
            width: self.width.to_font_units(options.shaper),
            ascent: self.ascent.to_font_units(options.shaper),
            descent: self.descent.to_font_units(options.shaper),
        };
        MathBox::empty(extents)
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for MathExpression<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        let mut math_box = self.content.layout(options);
        math_box.user_info = Some(self.user_info);
        math_box
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        self.content.operator_properties(options)
    }
}

impl<'a, T: 'a + Debug> MathBoxLayout<'a, T> for MathItem<T> {
    fn layout(self, options: LayoutOptions<'a>) -> MathBox<'a, T> {
        match self {
            MathItem::Field(field) => field.layout(options),
            MathItem::Space(space) => space.layout(options),
            MathItem::Atom(atom) => atom.layout(options),
            MathItem::GeneralizedFraction(frac) => frac.layout(options),
            MathItem::OverUnder(over_under) => over_under.layout(options),
            MathItem::List(list) => list.layout(options),
            MathItem::Root(root) => root.layout(options),
            MathItem::Operator(operator) => operator.layout(options),
        }
    }

    fn operator_properties(&self, options: LayoutOptions<'a>) -> Option<OperatorProperties> {
        match *self {
            MathItem::Field(ref field) => {
                MathBoxLayout::<'a, T>::operator_properties(field, options)
            }
            MathItem::Space(ref space) => {
                MathBoxLayout::<'a, T>::operator_properties(space, options)
            }
            MathItem::Atom(ref atom) => atom.operator_properties(options),
            MathItem::GeneralizedFraction(ref frac) => frac.operator_properties(options),
            MathItem::OverUnder(ref over_under) => over_under.operator_properties(options),
            MathItem::List(ref list) => list.operator_properties(options),
            MathItem::Root(ref root) => root.operator_properties(options),
            MathItem::Operator(ref operator) => {
                MathBoxLayout::<'a, T>::operator_properties(operator, options)
            }
        }
    }
}
