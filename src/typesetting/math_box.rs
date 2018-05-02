use std::cmp::{max, min};
use std::ops::{Add, Div, Mul, Sub};
use types::PercentValue;
use std::default::Default;

use typesetting::shaper::MathGlyph;

/// A point in 2D space.
///
/// Note: The y coordinate increases downwards.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Vector<T> {
    /// the x coordinate
    pub x: T,
    /// the y coordinate
    pub y: T,
}
impl Add<Vector<i32>> for Vector<i32> {
    type Output = Vector<i32>;
    fn add(self, _rhs: Vector<i32>) -> Vector<i32> {
        Vector {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
        }
    }
}
impl Sub<Vector<i32>> for Vector<i32> {
    type Output = Vector<i32>;
    fn sub(self, _rhs: Vector<i32>) -> Vector<i32> {
        Vector {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
        }
    }
}
impl Mul<i32> for Vector<i32> {
    type Output = Vector<i32>;
    fn mul(self, _rhs: i32) -> Vector<i32> {
        Vector {
            x: self.x * _rhs,
            y: self.y * _rhs,
        }
    }
}
impl Div<i32> for Vector<i32> {
    type Output = Vector<i32>;
    fn div(self, _rhs: i32) -> Vector<i32> {
        Vector {
            x: self.x / _rhs,
            y: self.y / _rhs,
        }
    }
}
impl Mul<PercentValue> for Vector<i32> {
    type Output = Vector<i32>;
    fn mul(self, _rhs: PercentValue) -> Vector<i32> {
        Vector {
            x: self.x * _rhs,
            y: self.y * _rhs,
        }
    }
}

/// Basic Extents of ink inside boxes
// TODO: Image for documentation
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Extents<T> {
    /// Horizontal offset from the left edge.
    pub left_side_bearing: T,
    /// Width
    pub width: T,
    /// Maximum extent above the baseline.
    pub ascent: T,
    /// Maximum extent below the baseline.
    pub descent: T,
}
impl Extents<i32> {
    pub fn new(left_side_bearing: i32, width: i32, ascent: i32, descent: i32) -> Self {
        Extents {
            left_side_bearing: left_side_bearing,
            width: width,
            ascent: ascent,
            descent: descent,
        }
    }
    /// Returns the height = ascent + descent of the box
    pub fn height(&self) -> i32 {
        self.ascent + self.descent
    }

    pub fn center(&self) -> i32 {
        (self.left_side_bearing + self.width) / 2
    }

    pub fn right_edge(&self) -> i32 {
        self.left_side_bearing + self.width
    }
}
impl Mul<i32> for Extents<i32> {
    type Output = Extents<i32>;
    fn mul(self, _rhs: i32) -> Extents<i32> {
        Extents {
            left_side_bearing: self.left_side_bearing * _rhs,
            width: self.width * _rhs,
            ascent: self.ascent * _rhs,
            descent: self.descent * _rhs,
        }
    }
}
impl Div<i32> for Extents<i32> {
    type Output = Extents<i32>;
    fn div(self, _rhs: i32) -> Extents<i32> {
        Extents {
            left_side_bearing: self.left_side_bearing / _rhs,
            width: self.width / _rhs,
            ascent: self.ascent / _rhs,
            descent: self.descent / _rhs,
        }
    }
}
impl Mul<PercentValue> for Extents<i32> {
    type Output = Extents<i32>;
    fn mul(self, _rhs: PercentValue) -> Extents<i32> {
        Extents {
            left_side_bearing: self.left_side_bearing * _rhs,
            width: self.width * _rhs,
            ascent: self.ascent * _rhs,
            descent: self.descent * _rhs,
        }
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct Moved<T> {
    pub offset: Vector<i32>,
    pub item: T,
}

/// Describes the box metrics for mathematical objects.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Bounds {
    /// Position on the left on the baseline.
    pub origin: Vector<i32>,
    /// Extents of the bounds.
    pub extents: Extents<i32>,
}
impl Bounds {
    #[allow(dead_code)]
    fn union_extents(self, other: Bounds) -> Extents<i32> {
        let max_x = max(
            self.origin.x + self.extents.width,
            other.origin.x + other.extents.width,
        );
        let min_x = min(self.origin.x, other.origin.x);
        let max_ascent = max(
            self.extents.ascent - self.origin.y,
            other.extents.ascent - other.origin.y,
        );
        let max_descent = max(
            self.extents.descent + self.origin.y,
            other.extents.descent + other.origin.y,
        );

        Extents {
            left_side_bearing: self.extents.left_side_bearing,
            width: max_x - min_x,
            ascent: max_ascent,
            descent: max_descent,
        }
    }
    /// Returns bounds that have non-negative ascent and descent by moving the origin.
    pub fn normalize(self) -> Bounds {
        assert!(self.extents.ascent >= -self.extents.descent);
        let mut result = self;
        if result.extents.descent < 0 {
            result.origin.y += result.extents.descent;
            result.extents.ascent += result.extents.descent;
            result.extents.descent = 0;
        }
        if result.extents.ascent < 0 {
            result.origin.y -= result.extents.ascent;
            result.extents.descent += result.extents.ascent;
            result.extents.ascent = 0;
        }
        result
    }
}

impl Mul<i32> for Bounds {
    type Output = Bounds;
    fn mul(self, _rhs: i32) -> Bounds {
        Bounds {
            origin: self.origin * _rhs,
            extents: self.extents * _rhs,
        }
    }
}
impl Div<i32> for Bounds {
    type Output = Bounds;
    fn div(self, _rhs: i32) -> Bounds {
        Bounds {
            origin: self.origin / _rhs,
            extents: self.extents / _rhs,
        }
    }
}

impl Mul<PercentValue> for Bounds {
    type Output = Bounds;
    fn mul(self, _rhs: PercentValue) -> Bounds {
        Bounds {
            origin: self.origin * _rhs,
            extents: self.extents * _rhs,
        }
    }
}

/// A box used in mathematical typesetting must have these metric values.
pub trait MathBoxMetrics {
    /// distance from the left edge of a box to the left edge of the following box
    fn advance_width(&self) -> i32;
    /// the size of a box
    fn extents(&self) -> Extents<i32>;
    /// extra advance width to apply if the following glyph is not italic
    fn italic_correction(&self) -> i32;
    /// the optical center above which to place an accent
    fn top_accent_attachment(&self) -> i32;
}

#[derive(Debug)]
pub enum Drawable<G: MathGlyph> {
    Glyph(G),
    Line { vector: Vector<i32>, thickness: u32 },
}

impl<'a, G: MathGlyph> MathBoxMetrics for Drawable<G> {
    fn advance_width(&self) -> i32 {
        match *self {
            Drawable::Glyph(glyph) => glyph.advance_width(),
            Drawable::Line { ref vector, .. } => vector.x,
        }
    }
    fn extents(&self) -> Extents<i32> {
        match *self {
            Drawable::Glyph(glyph) => glyph.extents(),
            Drawable::Line { ref vector, .. } => Extents {
                left_side_bearing: 0,
                width: vector.x,
                ascent: max(0, -vector.y),
                descent: max(0, vector.y),
            },
        }
    }

    fn italic_correction(&self) -> i32 {
        match *self {
            Drawable::Glyph(glyph) => glyph.italic_correction(),
            Drawable::Line { .. } => 0,
        }
    }

    fn top_accent_attachment(&self) -> i32 {
        let value = match *self {
            Drawable::Glyph(glyph) => glyph.top_accent_attachment(),
            _ => 0,
        };
        if value == 0 {
            self.advance_width() / 2
        } else {
            value
        }
    }
}

#[derive(Debug)]
pub enum MathBoxContent<G: MathGlyph> {
    /// Represents a box without any content
    Empty,
    Drawable(Drawable<G>),
    /// A vector of boxes that are logically inside the parent box.
    ///
    /// If this `Vec` is empty then thix box is considered empty.
    Boxes(Vec<MathBox<G>>),
}

pub struct MathBox<G: MathGlyph> {
    pub origin: Vector<i32>,
    content: MathBoxContent<G>,
}

impl<G: MathGlyph> Default for MathBoxContent<G> {
    fn default() -> Self {
        MathBoxContent::Empty
    }
}

impl<'a, G: MathGlyph> MathBoxMetrics for MathBoxContent<G> {
    fn advance_width(&self) -> i32 {
        match *self {
            MathBoxContent::Empty => 0,
            MathBoxContent::Drawable(ref drawable) => drawable.advance_width(),
            MathBoxContent::Boxes(ref boxes) => boxes
                .iter()
                .map(|&item| item.origin.x + item.advance_width())
                .max()
                .unwrap_or_default(),
        }
    }

    fn extents(&self) -> Extents<i32> {
        match *self {
            MathBoxContent::Empty => Extents::default(),
            MathBoxContent::Drawable(ref drawable) => drawable.extents(),
            MathBoxContent::Boxes(ref boxes) => {
                let slice = boxes.as_slice();
                let max_ascent = slice
                    .iter()
                    .map(|item| -item.origin.y + item.extents().ascent)
                    .max()
                    .unwrap_or_default();
                let max_descent = slice
                    .iter()
                    .map(|item| item.origin.y + item.extents().descent)
                    .max()
                    .unwrap_or_default();
                let left_side_bearing = slice
                    .get(0)
                    .map(|x| x.extents().left_side_bearing)
                    .unwrap_or(0);
                let width = slice
                    .iter()
                    .map(|item| {
                        item.origin.x + item.extents().left_side_bearing + item.extents().width
                    })
                    .max()
                    .unwrap_or(0) - left_side_bearing;
                Extents {
                    left_side_bearing: left_side_bearing,
                    width: width,
                    ascent: max_ascent,
                    descent: max_descent,
                }
            }
        }
    }

    fn italic_correction(&self) -> i32 {
        match *self {
            MathBoxContent::Empty => 0,
            MathBoxContent::Drawable(ref drawable) => drawable.italic_correction(),
            MathBoxContent::Boxes(ref boxes) => boxes
                .as_slice()
                .last()
                .map(|math_box| math_box.italic_correction())
                .unwrap_or_default(),
        }
    }

    fn top_accent_attachment(&self) -> i32 {
        let value = match *self {
            MathBoxContent::Drawable(ref drawable) => drawable.top_accent_attachment(),
            MathBoxContent::Boxes(ref boxes) if boxes.as_slice().len() == 1 => {
                boxes.as_slice().first().unwrap().top_accent_attachment()
            }
            _ => 0,
        };
        if value == 0 {
            self.advance_width() / 2
        } else {
            value
        }
    }
}

impl<G: MathGlyph> Default for MathBox<G> {
    fn default() -> Self {
        MathBox {
            origin: Vector::default(),
            content: Default::default(),
        }
    }
}

impl<G: MathGlyph> ::std::fmt::Debug for MathBox<G> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "MathBox")
    }
}

impl<G: MathGlyph> MathBox<G> {
    fn with_content(content: MathBoxContent<G>) -> Self {
        MathBox {
            content: content,
            origin: Vector::default(),
        }
    }

    pub fn empty(extents: Extents<i32>) -> Self {
        MathBox::with_content(MathBoxContent::Empty)
    }

    pub fn with_line(from: Vector<i32>, to: Vector<i32>, thickness: u32) -> Self {
        let mut math_box = MathBox::with_content(MathBoxContent::Drawable(Drawable::Line {
            vector: to - from,
            thickness: thickness,
        }));
        math_box.origin = from;
        math_box
    }

    pub fn with_glyph(glyph: G) -> Self {
        MathBox::with_content(MathBoxContent::Drawable(Drawable::Glyph(glyph)))
    }

    pub fn with_vec(vec: Vec<MathBox<G>>) -> Self {
        MathBox::with_content(MathBoxContent::Boxes(vec))
    }

    pub fn bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.content.extents(),
        }
    }

    pub(crate) fn content(&self) -> &MathBoxContent<G> {
        &self.content
    }

    /// recursive search for a glyph at the leftmost position
    pub fn first_glyph(&self) -> Option<&G> {
        match *self.content() {
            MathBoxContent::Drawable(Drawable::Glyph(ref glyph)) => Some(glyph),
            MathBoxContent::Boxes(ref boxes) => boxes.first().and_then(|node| node.first_glyph()),
            _ => None,
        }
    }

    pub fn last_glyph(&self) -> Option<&G> {
        match *self.content() {
            MathBoxContent::Drawable(Drawable::Glyph(ref glyph)) => Some(glyph),
            MathBoxContent::Boxes(ref boxes) => boxes.last().and_then(|node| node.last_glyph()),
            _ => None,
        }
    }
}

impl<G: MathGlyph> MathBoxMetrics for MathBox<G> {
    fn advance_width(&self) -> i32 {
        self.content.advance_width()
    }

    fn extents(&self) -> Extents<i32> {
        self.content.extents()
    }

    fn italic_correction(&self) -> i32 {
        self.content.italic_correction()
    }

    fn top_accent_attachment(&self) -> i32 {
        self.content.top_accent_attachment()
    }
}
