#![allow(unused_variables, dead_code)]
extern crate freetype;

use types::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::iter;
use std::iter::IntoIterator;
use std::cmp::max;

use super::font::{MathFont, hb};
use super::shaper::MathShaper;
use super::math_box::{MathBox, Content, Extents, Point};
use super::multiscripts::*;

pub type ListIter = Box<Iterator<Item = ListItem>>;
pub type BoxIter = Box<Iterator<Item = MathBox>>;

#[derive(Clone)]
pub struct LayoutOptions<'a> {
    pub font: &'a MathFont<'a>,
    pub shaper: Rc<RefCell<MathShaper>>,
    pub style: MathStyle,

    pub ft_library: &'a freetype::Library,
}

pub trait MathBoxLayout {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a>;
}

impl<I> MathBoxLayout for I
    where I: 'static + IntoIterator<Item=ListItem>
{
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        let mut cursor = 0i32;
        let mut previous_ital_cor = 0;
        let layouted = self.into_iter().map(move |item| {
            let mut math_box: MathBox = item.layout(options.clone()).collect();
            if math_box.italic_correction == 0 {
                if previous_ital_cor != 0 {
                    cursor += previous_ital_cor;
                }
                previous_ital_cor = 0;
            } else {
                previous_ital_cor = math_box.italic_correction;
            }
            math_box.origin.x = cursor;
            cursor += math_box.logical_extents.width;
            math_box
        });
        Box::new(layouted)
    }
}

impl MathBoxLayout for Atom {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        assert!(self.has_nucleus());
        if !self.has_any_attachments() {
            return self.nucleus.layout(options);
        }
        if self.has_top_right() {
            let mut superscript_options = options.clone();
            superscript_options.style = options.style.superscript_style();
            return layout_superscript(self.top_right.layout(superscript_options).collect(),
                               self.nucleus.layout(options.clone()).collect(),
                               &options.font,
                               options.style)
        } else {
            unimplemented!()
        }

    }
}


fn layout_superscript(mut superscript: MathBox,
                      nucleus: MathBox,
                      font: &MathFont,
                      style: MathStyle)
                      -> BoxIter {
    let space_after_script = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_SPACE_AFTER_SCRIPT);

    let superscript_shift_up = get_superscript_shift_up(&superscript, &nucleus, font, style);

    let superscript_kerning = get_attachment_kern(&nucleus,
                                                  &superscript,
                                                  CornerPosition::TopRight,
                                                  superscript_shift_up,
                                                  font);

    superscript.origin.x  = nucleus.origin.x + nucleus.logical_extents.width;
    superscript.origin.x += superscript_kerning;
    superscript.origin.y -= superscript_shift_up;
    superscript.logical_extents.width += space_after_script;
    let result = vec![nucleus, superscript];
    Box::new(result.into_iter())
}

impl MathBoxLayout for OverUnder {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        if self.has_over() {
            let mut over_options = options.clone();
            if !self.over_is_accent {
                over_options.style = over_options.style.superscript_style();
            }
            let font = &options.font;
            let style = options.style;
            layout_over(self.over.layout(over_options).collect(), self.nucleus.layout(options.clone()).collect(), font, style, self.over_is_accent)
        } else {
            unimplemented!()
        }
    }
}

fn layout_over(mut over: MathBox, mut nucleus: MathBox, font: &MathFont, style: MathStyle, as_accent: bool) -> BoxIter {
    let over_gap = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_OVERBAR_VERTICAL_GAP);
    let over_shift = over_gap + nucleus.ink_extents.ascent + over.ink_extents.descent;
    println!("{:?}", over);
    over.origin.y -= over_shift;

    // centering
    let width_difference = nucleus.ink_extents.width - over.ink_extents.width;
    if width_difference < 0 {
        nucleus.origin.x -= width_difference / 2;
    } else {
        over.origin.x += width_difference / 2;
    }

    // first the over than the nucleus to preserve its italic collection
    Box::new(vec![over, nucleus].into_iter())
}

impl MathBoxLayout for GeneralizedFraction {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        let mut fraction_options = options.clone();
        fraction_options.style = fraction_options.style.primed_style();
        let mut numerator: MathBox = self.numerator.layout(fraction_options.clone()).collect();
        let mut denominator: MathBox = self.denominator.layout(fraction_options.clone()).collect();
        let font = &options.font;

        let axis_height = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_AXIS_HEIGHT);
        let default_thickness = font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_RULE_THICKNESS);

        let (numerator_shift_up, denominator_shift_dn) = if options.style <= MathStyle::TextStyle {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_SHIFT_UP),
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_SHIFT_DOWN))
        } else {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_DISPLAY_STYLE_SHIFT_UP),
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_DISPLAY_STYLE_SHIFT_DOWN))
        };

        let (numerator_gap_min, denominator_gap_min) = if options.style <= MathStyle::TextStyle {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUMERATOR_GAP_MIN),
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOMINATOR_GAP_MIN))
        } else {
            (font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_NUM_DISPLAY_STYLE_GAP_MIN),
            font.get_math_constant(hb::HB_OT_MATH_CONSTANT_FRACTION_DENOM_DISPLAY_STYLE_GAP_MIN))
        };

        let numerator_shift_up = max(numerator_shift_up - axis_height, numerator_gap_min + default_thickness / 2 + numerator.ink_extents.descent);
        let denominator_shift_dn = max(denominator_shift_dn + axis_height, denominator_gap_min + default_thickness / 2 + denominator.ink_extents.ascent);

        numerator.origin.y -= axis_height;
        denominator.origin.y -= axis_height;

        numerator.origin.y -= numerator_shift_up;
        denominator.origin.y += denominator_shift_dn;

        // centering
        let width_difference = numerator.ink_extents.width - denominator.ink_extents.width;
        if width_difference < 0 {
            numerator.origin.x -= width_difference / 2;
        } else {
            denominator.origin.x += width_difference / 2;
        }

        let fraction_bar_extents = Extents { width: max(numerator.ink_extents.width, denominator.ink_extents.width),
                                             ascent: default_thickness, descent: 0 };
        let fraction_bar = MathBox { origin: Point { x: 0, y: - axis_height + default_thickness / 2 },
                                     ink_extents: fraction_bar_extents,
                                     logical_extents: fraction_bar_extents,
                                     content: Content::Filled, ..Default::default() };

        Box::new(vec![numerator, fraction_bar, denominator].into_iter())
    }
}

impl MathBoxLayout for Field {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        match self {
            Field::Empty => Box::new(iter::empty::<MathBox>()),
            Field::Glyph(..) => unreachable!(),
            Field::Unicode(content) => {
                let mut shaper = options.shaper.borrow_mut();
                let shaping_result = shaper.shape(&content, &options.font, options.style);
                Box::new(shaping_result.into_iter())
            }
            Field::List(list) => list.into_iter().layout(options),
        }
    }
}

impl MathBoxLayout for ListItem {
    fn layout<'a>(self, options: LayoutOptions<'a>) -> Box<Iterator<Item=MathBox> + 'a> {
        match self {
            ListItem::Atom(atom) => atom.layout(options),
            ListItem::GeneralizedFraction(frac) => frac.layout(options),
            ListItem::OverUnder(over_under) => over_under.layout(options),
            _ => unimplemented!(),
        }
    }
}
