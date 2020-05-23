#![allow(unused_variables, dead_code)]
use crate::types::*;
use std::cmp::{max, min};

use super::math_box::{Extents, MathBox, MathBoxMetrics, Vector};
use super::multiscripts::*;
use super::shaper::{MathConstant, MathShaper};
use super::stretchy::*;

#[derive(Copy, Clone)]
pub struct LayoutOptions<'a> {
    pub shaper: &'a dyn MathShaper,
    pub style: LayoutStyle,
    pub stretch_size: Option<Extents<i32>>,
    pub user_data: u64,
}

impl<'a> LayoutOptions<'a> {
    pub fn user_data(self, user_data: u64) -> Self {
        LayoutOptions { user_data, ..self }
    }
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
    pub is_large_op: bool,
}

impl Length {
    fn to_font_units(self, shaper: &dyn MathShaper) -> i32 {
        if self.is_null() {
            return 0;
        }
        match self.unit {
            LengthUnit::Em => (shaper.em_size() as f32 * self.value) as i32,
            LengthUnit::Point => {
                Length::em(self.value / shaper.ppem().0 as f32).to_font_units(shaper)
            }
            LengthUnit::DisplayOperatorMinHeight => {
                (shaper.math_constant(MathConstant::DisplayOperatorMinHeight) as f32 * self.value)
                    as i32
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

/// The trait that every Item in a math list satisfies so that the entire math list can be
/// laid out.
pub trait MathLayout: ::std::fmt::Debug {
    fn layout(&self, options: LayoutOptions) -> MathBox;
    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        None
    }
    fn can_stretch(&self, options: LayoutOptions) -> bool {
        self.operator_properties(options)
            .map(|operator_properties| operator_properties.stretch_properties.is_some())
            .unwrap_or_default()
    }
    fn is_large_op(&self, options: LayoutOptions) -> bool {
        self.operator_properties(options)
            .map(|operator_properties| operator_properties.is_large_op)
            .unwrap_or_default()
    }
}

impl MathLayout for Field {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        match *self {
            Field::Empty => MathBox::default(),
            Field::Glyph(ref glyph) => unimplemented!(),
            Field::Unicode(ref content) => {
                let shaper = options.shaper;
                shaper.shape(&content, options.style, options.user_data)
            }
        }
    }
}

impl MathLayout for [MathExpression] {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let boxes = layout_strechy_list(self, options);

        let mut cursor = 0i32;
        let mut previout_italic_correction = 0;
        let layouted = boxes.into_iter().map(move |mut math_box| {
            // apply italic correction if current glyph is upright
            if math_box.italic_correction() == 0 {
                cursor += previout_italic_correction;
            }
            math_box.origin.x += cursor;
            cursor += math_box.advance_width();
            previout_italic_correction = math_box.italic_correction();
            math_box
        });
        MathBox::with_vec(layouted.collect(), options.user_data)
    }
}

impl MathLayout for Vec<MathExpression> {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        self.as_slice().layout(options)
    }
}

impl MathLayout for Atom {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let subscript = self.bottom_right.as_ref();
        let superscript = self.top_right.as_ref();
        let nucleus = self.nucleus.as_ref();
        layout_sub_superscript(subscript, superscript, nucleus, options)
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        self.nucleus
            .as_ref()
            .and_then(|nucleus| nucleus.operator_properties(options))
    }
}

fn layout_sub_superscript(
    subscript: Option<&MathExpression>,
    superscript: Option<&MathExpression>,
    nucleus: Option<&MathExpression>,
    options: LayoutOptions,
) -> MathBox {
    let nucleus = match nucleus {
        Some(nucleus) => nucleus,
        None => return MathBox::empty(Extents::default(), options.user_data),
    };
    let subscript_options = LayoutOptions {
        style: options.style.subscript_style(),
        ..options
    };
    let superscript_options = LayoutOptions {
        style: options.style.superscript_style(),
        ..options
    };
    let subscript = subscript.map(|x| x.layout(subscript_options));
    let superscript = superscript.map(|x| x.layout(superscript_options));
    let nucleus_is_largeop = nucleus.is_large_op(options);
    let mut nucleus = nucleus.layout(options);

    let space_after_script = options.shaper.math_constant(MathConstant::SpaceAfterScript);

    if subscript.is_none() && superscript.is_none() {
        return nucleus;
    }

    let mut result = Vec::with_capacity(4);
    match (subscript, superscript) {
        (Some(mut subscript), Some(mut superscript)) => {
            let (sub_shift, super_shift) =
                get_subsup_shifts(&subscript, &superscript, &nucleus, options);
            position_attachment(
                &mut subscript,
                &mut nucleus,
                nucleus_is_largeop,
                CornerPosition::BottomRight,
                sub_shift,
                options,
            );
            position_attachment(
                &mut superscript,
                &mut nucleus,
                nucleus_is_largeop,
                CornerPosition::TopRight,
                super_shift,
                options,
            );
            result.push(nucleus);
            result.push(subscript);
            result.push(superscript);
        }
        (Some(mut subscript), None) => {
            let sub_shift = get_subscript_shift_dn(&subscript, &nucleus, options);
            position_attachment(
                &mut subscript,
                &mut nucleus,
                nucleus_is_largeop,
                CornerPosition::BottomRight,
                sub_shift,
                options,
            );
            result.push(nucleus);
            result.push(subscript);
        }
        (None, Some(mut superscript)) => {
            let super_shift = get_superscript_shift_up(&superscript, &nucleus, options);
            position_attachment(
                &mut superscript,
                &mut nucleus,
                nucleus_is_largeop,
                CornerPosition::TopRight,
                super_shift,
                options,
            );
            result.push(nucleus);
            result.push(superscript);
        }
        // we dealt with this case earlier
        (None, None) => unreachable!(),
    }

    let mut space = MathBox::empty(Extents::new(0, space_after_script, 0, 0), options.user_data);
    space.origin.x = result
        .iter()
        .map(|math_box| math_box.origin.x + math_box.advance_width())
        .max()
        .unwrap_or_default();
    result.push(space);

    MathBox::with_vec(result, options.user_data)
}

impl MathLayout for OverUnder {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let nucleus = match self.nucleus {
            Some(ref nucleus) => nucleus,
            None => return MathBox::empty(Extents::default(), options.user_data),
        };

        // Display `OverUnder` like an `Atom` if we want to render limits and the current style is
        // inline style.
        if self.is_limits && options.style.math_style == MathStyle::Inline {
            return layout_sub_superscript(
                self.under.as_ref(),
                self.over.as_ref(),
                Some(&nucleus),
                options,
            );
        }

        let nucleus_is_largeop = nucleus.is_large_op(options);
        let nucleus_is_horizontally_stretchy = nucleus.can_stretch(options);

        let mut over_options = LayoutOptions {
            style: options.style.inline_style().no_flat_accent_style(),
            stretch_size: None,
            ..options
        };
        if !self.over_is_accent {
            over_options.style = over_options.style.superscript_style();
        }
        let mut under_options = LayoutOptions {
            style: options.style.inline_style().no_flat_accent_style(),
            stretch_size: None,
            ..options
        };
        if !self.under_is_accent {
            under_options.style = under_options.style.subscript_style();
        }
        let mut arguments = [
            (Some(nucleus), options, false),
            (self.over.as_ref(), over_options, self.over_is_accent),
            (self.under.as_ref(), under_options, self.under_is_accent),
        ];
        let mut boxes = [None, None, None];

        for (index, &mut (ref mut arg, options, ..)) in arguments.iter_mut().enumerate() {
            // first take and layout non-stretchy subexpressions
            if !arg
                .as_ref()
                .map(|x| x.can_stretch(options))
                .unwrap_or(false)
            {
                boxes[index] = arg.map(|arg| arg.layout(options));
            }
        }
        // get the maximal width of the non-stretchy items
        let mut max_width = boxes
            .iter()
            .map(|math_box| {
                math_box
                    .as_ref()
                    .map(|x| x.extents().width)
                    .unwrap_or_default()
            })
            .max()
            .unwrap_or_default();

        // the OverUnder has to stretch to at least the current stretch size
        if let Some(Extents {
            width: stretch_width,
            ..
        }) = options.stretch_size
        {
            max_width = max(max_width, stretch_width);
        }

        // layout the rest
        for (index, &mut (ref mut arg, ref mut options, as_accent)) in
            arguments.iter_mut().enumerate()
        {
            let mut stretch_size = options.stretch_size.unwrap_or(Default::default());
            stretch_size.width = max_width;
            options.stretch_size = Some(stretch_size);

            options.style.as_accent = as_accent;
            if let Some(stretched_box) = arg.map(|arg| arg.layout(*options)) {
                boxes[index] = Some(stretched_box);
            }
        }

        let nucleus = boxes[0].take().unwrap_or_default();
        let nucleus = if let Some(mut over) = boxes[1].take() {
            let (_, LayoutOptions { style, shaper, .. }, ..) = arguments[1];

            // enable flat accents if needed
            let height = options
                .shaper
                .math_constant(MathConstant::FlattenedAccentBaseHeight);
            if self.over_is_accent && nucleus.extents().ascent >= height {
                let (_, ref mut over_options, _) = arguments[1];
                over_options.style.flat_accent = true;
                over = self.over.as_ref().unwrap().layout(*over_options);
            }

            layout_over_or_under(
                over,
                nucleus,
                options,
                true,
                self.over_is_accent,
                nucleus_is_largeop,
                nucleus_is_horizontally_stretchy,
            )
        } else {
            nucleus
        };

        if let Some(under) = boxes[2].take() {
            let (_, LayoutOptions { style, shaper, .. }, ..) = arguments[2];
            layout_over_or_under(
                under,
                nucleus,
                options,
                false,
                self.under_is_accent,
                nucleus_is_largeop,
                nucleus_is_horizontally_stretchy,
            )
        } else {
            nucleus
        }
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        self.nucleus
            .as_ref()
            .and_then(|nucleus| nucleus.operator_properties(options))
    }
}

fn layout_over_or_under(
    mut attachment: MathBox,
    mut nucleus: MathBox,
    options: LayoutOptions,
    as_over: bool,
    as_accent: bool,
    nucleus_is_large_op: bool,
    nucleus_is_horizontally_stretchy: bool,
) -> MathBox {
    let (shaper, style) = (options.shaper, options.style);
    let mut gap = 0;
    let mut shift = 0;
    if nucleus_is_large_op {
        if as_over {
            gap = shaper.math_constant(MathConstant::UpperLimitGapMin);
            shift = shaper.math_constant(MathConstant::UpperLimitBaselineRiseMin)
                + nucleus.extents().ascent;
        } else {
            gap = shaper.math_constant(MathConstant::LowerLimitGapMin);
            shift = shaper.math_constant(MathConstant::LowerLimitBaselineDropMin)
                + nucleus.extents().descent;
        }
    } else if nucleus_is_horizontally_stretchy {
        if as_over {
            gap = shaper.math_constant(MathConstant::StretchStackGapBelowMin);
            shift = shaper.math_constant(MathConstant::StretchStackTopShiftUp);
        } else {
            gap = shaper.math_constant(MathConstant::StretchStackGapAboveMin);
            shift = shaper.math_constant(MathConstant::StretchStackBottomShiftDown);
        }
    } else if !as_accent {
        gap = if as_over {
            shaper.math_constant(MathConstant::OverbarVerticalGap)
        } else {
            shaper.math_constant(MathConstant::UnderbarVerticalGap)
        };
        shift = gap;
    }

    let baseline_offset = if as_accent {
        if as_over {
            let accent_base_height = shaper.math_constant(MathConstant::AccentBaseHeight);
            -max(nucleus.extents().ascent - accent_base_height, 0)
        } else {
            nucleus.extents().descent
        }
    } else {
        if as_over {
            -max(
                shift,
                nucleus.extents().ascent + gap + attachment.extents().descent,
            )
        } else {
            max(
                shift,
                nucleus.extents().descent + gap + attachment.extents().ascent,
            )
        }
    };

    attachment.origin.y += nucleus.origin.y;
    attachment.origin.y += baseline_offset;

    // centering
    let center_difference = if as_accent && as_over {
        (nucleus.origin.x + nucleus.top_accent_attachment())
            - (attachment.origin.x + attachment.top_accent_attachment())
    } else {
        (nucleus.origin.x + nucleus.extents().center())
            - (attachment.origin.x + attachment.extents().center())
    };
    if center_difference < 0 && !as_accent {
        nucleus.origin.x -= center_difference;
    } else {
        attachment.origin.x += center_difference;
    }

    // LargeOp italic correction
    if nucleus_is_large_op {
        if as_over {
            attachment.origin.x += nucleus.italic_correction() / 2;
        } else {
            attachment.origin.x -= nucleus.italic_correction() / 2;
        }
    }

    let advance_width = if as_accent {
        nucleus.advance_width()
    } else {
        max(
            nucleus.origin.x + nucleus.advance_width(),
            attachment.origin.x + attachment.advance_width(),
        )
    };
    let italic_correction = if as_accent {
        nucleus.italic_correction()
    } else {
        0
    };
    let top_accent_attachment = if as_over {
        attachment.origin.x + attachment.top_accent_attachment()
    } else {
        nucleus.origin.x + nucleus.top_accent_attachment()
    };

    let mut math_box = MathBox::with_vec(vec![nucleus, attachment], options.user_data);
    // preserve the italic collection of the nucleus
    math_box.metrics.advance_width = advance_width;
    math_box.metrics.italic_correction = italic_correction;
    math_box.metrics.top_accent_attachment = top_accent_attachment;

    math_box
}

impl MathLayout for GeneralizedFraction {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let (numerator, denominator) = match (&self.numerator, &self.denominator) {
            (&Some(ref a), &Some(ref b)) => (a, b),
            _ => return MathBox::default(),
        };

        let mut numerator_options = options;
        if options.style.math_style == MathStyle::Display {
            numerator_options.style.math_style = MathStyle::Inline;
        } else {
            numerator_options.style.script_level += 1;
        }
        let denominator_options = LayoutOptions {
            style: numerator_options.style.cramped_style(),
            ..options
        };
        let mut numerator = numerator.layout(numerator_options);
        let mut denominator = denominator.layout(denominator_options);

        let shaper = &options.shaper;
        let axis_height = shaper.math_constant(MathConstant::AxisHeight);
        let default_thickness = shaper.math_constant(MathConstant::FractionRuleThickness);

        let (numerator_shift_up, denominator_shift_dn) =
            if options.style.math_style == MathStyle::Inline {
                (
                    shaper.math_constant(MathConstant::FractionNumeratorShiftUp),
                    shaper.math_constant(MathConstant::FractionDenominatorShiftDown),
                )
            } else {
                (
                    shaper.math_constant(MathConstant::FractionNumeratorDisplayStyleShiftUp),
                    shaper.math_constant(MathConstant::FractionDenominatorDisplayStyleShiftDown),
                )
            };

        let (numerator_gap_min, denominator_gap_min) =
            if options.style.math_style == MathStyle::Inline {
                (
                    shaper.math_constant(MathConstant::FractionNumeratorGapMin),
                    shaper.math_constant(MathConstant::FractionDenominatorGapMin),
                )
            } else {
                (
                    shaper.math_constant(MathConstant::FractionNumDisplayStyleGapMin),
                    shaper.math_constant(MathConstant::FractionDenomDisplayStyleGapMin),
                )
            };

        let numerator_shift_up = max(
            numerator_shift_up - axis_height,
            numerator_gap_min + default_thickness / 2 + numerator.extents().descent,
        );
        let denominator_shift_dn = max(
            denominator_shift_dn + axis_height,
            denominator_gap_min + default_thickness / 2 + denominator.extents().ascent,
        );

        numerator.origin.y -= axis_height;
        denominator.origin.y -= axis_height;

        numerator.origin.y -= numerator_shift_up;
        denominator.origin.y += denominator_shift_dn;

        // centering
        let center_difference = (numerator.origin.x + numerator.extents().center())
            - (denominator.origin.x + denominator.extents().center());
        if center_difference < 0 {
            numerator.origin.x -= center_difference;
        } else {
            denominator.origin.x += center_difference;
        }

        // the fraction rule
        let origin = Vector {
            x: min(
                numerator.origin.x + numerator.extents().left_side_bearing,
                denominator.origin.x + denominator.extents().left_side_bearing,
            ),
            y: -axis_height,
        };
        let target = Vector {
            x: max(
                numerator.origin.x + numerator.extents().right_edge(),
                denominator.origin.x + denominator.extents().right_edge(),
            ),
            ..origin
        };
        let fraction_rule =
            MathBox::with_line(origin, target, default_thickness as u32, options.user_data);

        MathBox::with_vec(
            vec![numerator, fraction_rule, denominator],
            options.user_data,
        )
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        self.numerator
            .as_ref()
            .and_then(|numerator| numerator.operator_properties(options))
    }
}

impl MathLayout for Root {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let radicand = match &self.radicand {
            Some(x) => x,
            _ => return MathBox::default(),
        };

        let shaper = options.shaper;
        let line_thickness = shaper.math_constant(MathConstant::RadicalRuleThickness);
        let vertical_gap = if options.style.math_style == MathStyle::Inline {
            shaper.math_constant(MathConstant::RadicalVerticalGap)
        } else {
            shaper.math_constant(MathConstant::RadicalDisplayStyleVerticalGap)
        };
        let extra_ascender = shaper.math_constant(MathConstant::RadicalExtraAscender);

        // calculate the needed surd height based on the height of the radicand
        let mut radicand = radicand.layout(options);
        let needed_surd_height = radicand.extents().height() + vertical_gap + line_thickness;

        // draw a stretched version of the surd
        // let surd_style = LayoutStyle {
        //     stretch_constraints: Some(Vector {
        //         x: 0,
        //         y: needed_surd_height,
        //     }),
        //     ..options.style
        // };
        let surd = options.shaper.shape("√", options.style, options.user_data);
        let mut surd = surd
            .first_glyph()
            .and_then(|(glyph, _scale)| {
                if options.shaper.is_stretchable(glyph.glyph_code, false) {
                    Some(options.shaper.stretch_glyph(
                        glyph.glyph_code,
                        false,
                        needed_surd_height.abs() as u32,
                        options.style,
                        options.user_data,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // raise the surd so that its ascent is at least the radicand's ascender plus the radical
        // gap plus the line thickness of the radical rule
        let surd_excess_height =
            surd.extents().height() - (radicand.extents().height() + vertical_gap + line_thickness);

        surd.origin.y =
            (radicand.extents().descent - surd.extents().descent) + surd_excess_height / 2;

        // place the radicand after the surd
        radicand.origin.x += surd.origin.x + surd.advance_width();

        // the radical rule
        let origin = Vector {
            x: surd.origin.x + surd.advance_width(),
            y: surd.origin.y - surd.extents().ascent + line_thickness / 2,
        };
        let target = Vector {
            x: origin.x + radicand.extents().right_edge(),
            ..origin
        };
        let mut radical_rule =
            MathBox::with_line(origin, target, line_thickness as u32, options.user_data);

        let mut boxes = vec![];

        // typeset the self degree
        if let &Some(ref degree) = &self.degree {
            let degree_bottom_raise_percent = PercentValue::new(
                shaper.math_constant(MathConstant::RadicalDegreeBottomRaisePercent) as u8,
            );
            let kern_before = shaper.math_constant(MathConstant::RadicalKernBeforeDegree);
            let kern_after = shaper.math_constant(MathConstant::RadicalKernAfterDegree);
            let surd_height = surd.extents().ascent + surd.extents().descent;
            let degree_bottom =
                surd.origin.y + surd.extents().descent - surd_height * degree_bottom_raise_percent;

            let mut degree_options = options;
            degree_options.style.script_level += 2;
            degree_options.style.math_style = MathStyle::Inline;
            let mut degree = degree.layout(degree_options);
            degree.origin.y += degree_bottom;
            degree.origin.x += kern_before;

            let surd_kern = kern_before + degree.advance_width() + kern_after;
            surd.origin.x += surd_kern;
            radicand.origin.x += surd_kern;
            radical_rule.origin.x += surd_kern;

            boxes.push(degree);
        }

        boxes.append(&mut vec![surd, radical_rule, radicand]);
        MathBox::with_vec(boxes, options.user_data)
        // TODO
        // let mut combined_box = boxes.into_iter().collect::<MathBox>();
        // combined_box.logical_extents.ascent += extra_ascender;
        // Box::new(iter::once(combined_box))
    }
}

impl Operator {
    fn layout_stretchy(
        &self,
        needed_height: u32,
        needed_width: u32,
        options: LayoutOptions,
    ) -> MathBox {
        match self.field {
            Field::Unicode(ref string) => {
                let shape_result = options.shaper.shape(
                    string,
                    options.style.no_flat_accent_style(),
                    options.user_data,
                );
                let first_glyph = match shape_result.first_glyph() {
                    Some((glyph, _scale)) => glyph,
                    None => return MathBox::empty(Extents::default(), options.user_data),
                };

                if needed_width > 0 && options.shaper.is_stretchable(first_glyph.glyph_code, true) {
                    return options.shaper.stretch_glyph(
                        first_glyph.glyph_code,
                        true,
                        needed_width,
                        options.style,
                        options.user_data,
                    );
                }

                if needed_height > 0 && options.shaper.is_stretchable(first_glyph.glyph_code, false)
                {
                    let mut math_box = options.shaper.stretch_glyph(
                        first_glyph.glyph_code,
                        false,
                        needed_height,
                        options.style,
                        options.user_data,
                    );
                    let stretch_constraints =
                        self.stretch_constraints.unwrap_or(StretchConstraints {
                            symmetric: true,
                            ..Default::default()
                        });
                    if stretch_constraints.symmetric {
                        let axis_height = options.shaper.math_constant(MathConstant::AxisHeight);
                        let shift_up = (math_box.extents().descent - math_box.extents().ascent) / 2
                            + axis_height;
                        math_box.origin.y -= shift_up;
                    } else {
                        let stretch_size = options.stretch_size.unwrap_or_default();
                        let excess_ascent = math_box.extents().ascent - stretch_size.ascent;
                        let excess_descent = math_box.extents().descent - stretch_size.descent;
                        math_box.origin.y += (excess_ascent - excess_descent) / 2;
                    }

                    return math_box;
                }

                // fallback
                options
                    .shaper
                    .shape(string, options.style, options.user_data)
            }
            _ => unimplemented!(),
        }
    }
}

impl MathLayout for Operator {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        match (options.stretch_size, self.stretch_constraints) {
            (Some(stretch_size), Some(stretch_constraints)) => {
                let min_size = stretch_constraints
                    .min_size
                    .map(|size| size.to_font_units(options.shaper));
                let max_size = stretch_constraints
                    .max_size
                    .map(|size| size.to_font_units(options.shaper));
                let mut needed_height = if stretch_constraints.symmetric {
                    let axis_height = options.shaper.math_constant(MathConstant::AxisHeight);
                    max(
                        stretch_size.ascent - axis_height,
                        axis_height + stretch_size.descent,
                    ) * 2
                } else {
                    stretch_size.ascent + stretch_size.descent
                };
                needed_height = clamp(needed_height, min_size, max_size);
                let needed_height = max(0, needed_height) as u32;
                self.layout_stretchy(needed_height, stretch_size.width as u32, options)
            }
            _ => {
                if self.is_large_op && options.style.math_style == MathStyle::Display {
                    let display_min_height = (options
                        .shaper
                        .math_constant(MathConstant::DisplayOperatorMinHeight)
                        as f32
                        * 1.42) as i32;
                    self.layout_stretchy(display_min_height as u32, 0, options)
                } else {
                    self.field.layout(options)
                }
            }
        }
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        Some(OperatorProperties {
            stretch_properties: self
                .stretch_constraints
                .as_ref()
                .map(|_| Default::default()),
            leading_space: self.leading_space.to_font_units(options.shaper),
            trailing_space: self.trailing_space.to_font_units(options.shaper),
            is_large_op: self.is_large_op,
        })
    }
}

impl MathLayout for MathSpace {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        let extents = Extents {
            left_side_bearing: 0,
            width: self.width.to_font_units(options.shaper),
            ascent: self.ascent.to_font_units(options.shaper),
            descent: self.descent.to_font_units(options.shaper),
        };
        MathBox::empty(extents, options.user_data)
    }
}

impl MathLayout for Option<MathExpression> {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        match *self {
            Some(ref item) => item.layout(options),
            None => MathBox::empty(Extents::default(), options.user_data),
        }
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        self.as_ref()
            .and_then(|node| node.operator_properties(options))
    }
}

impl MathLayout for MathItem {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        match *self {
            MathItem::Field(ref field) => field.layout(options),
            MathItem::Space(ref space) => space.layout(options),
            MathItem::Atom(ref atom) => atom.layout(options),
            MathItem::GeneralizedFraction(ref frac) => frac.layout(options),
            MathItem::OverUnder(ref over_under) => over_under.layout(options),
            MathItem::Root(ref root) => root.layout(options),
            MathItem::Operator(ref operator) => operator.layout(options),
            MathItem::List(ref list) => list.layout(options),
            MathItem::Other(ref other) => other.layout(options),
        }
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        match *self {
            MathItem::Field(ref field) => field.operator_properties(options),
            MathItem::Space(ref space) => space.operator_properties(options),
            MathItem::Atom(ref atom) => atom.operator_properties(options),
            MathItem::GeneralizedFraction(ref frac) => frac.operator_properties(options),
            MathItem::OverUnder(ref over_under) => over_under.operator_properties(options),
            MathItem::List(ref list) => (&list[..]).operator_properties(options),
            MathItem::Root(ref root) => root.operator_properties(options),
            MathItem::Operator(ref operator) => operator.operator_properties(options),
            MathItem::Other(ref other) => other.operator_properties(options),
        }
    }
}

pub fn layout_expression(expr: &MathExpression, options: LayoutOptions) -> MathBox {
    expr.layout(options)
}

impl MathLayout for MathExpression {
    fn layout(&self, options: LayoutOptions) -> MathBox {
        self.item.layout(options.user_data(self.get_user_data()))
    }

    fn operator_properties(&self, options: LayoutOptions) -> Option<OperatorProperties> {
        self.item.operator_properties(options)
    }
}
