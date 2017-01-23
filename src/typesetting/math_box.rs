use std;
use std::cmp::{max, min};
use std::ops::{Mul, Div, Add, Sub};
use types::{Glyph, PercentScale, PercentScale2D};
use std::default::Default;

use std::cell::Cell;
use typesetting::shaper::MathShaper;

use super::lazy_vec;
use super::lazy_vec::LazyVec;

/// A point in 2D space.
///
/// Note: The y coordinate increases downwards.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Point<T> {
    /// the x coordinate
    pub x: T,
    /// the y coordinate
    pub y: T,
}
impl Add<Point<i32>> for Point<i32> {
    type Output = Point<i32>;
    fn add(self, _rhs: Point<i32>) -> Point<i32> {
        Point {
            x: self.x + _rhs.x,
            y: self.y + _rhs.y,
        }
    }
}
impl Sub<Point<i32>> for Point<i32> {
    type Output = Point<i32>;
    fn sub(self, _rhs: Point<i32>) -> Point<i32> {
        Point {
            x: self.x - _rhs.x,
            y: self.y - _rhs.y,
        }
    }
}
impl Mul<i32> for Point<i32> {
    type Output = Point<i32>;
    fn mul(self, _rhs: i32) -> Point<i32> {
        Point {
            x: self.x * _rhs,
            y: self.y * _rhs,
        }
    }
}
impl Div<i32> for Point<i32> {
    type Output = Point<i32>;
    fn div(self, _rhs: i32) -> Point<i32> {
        Point {
            x: self.x / _rhs,
            y: self.y / _rhs,
        }
    }
}
impl Mul<PercentScale> for Point<i32> {
    type Output = Point<i32>;
    fn mul(self, _rhs: PercentScale) -> Point<i32> {
        Point {
            x: self.x * _rhs,
            y: self.y * _rhs,
        }
    }
}
impl Mul<PercentScale2D> for Point<i32> {
    type Output = Point<i32>;
    fn mul(self, _rhs: PercentScale2D) -> Point<i32> {
        Point {
            x: self.x * _rhs.horiz,
            y: self.y * _rhs.vert,
        }
    }
}

/// Basic Extents of boxes
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Extents<T> {
    /// Width of the box
    pub width: T,
    /// Maximum extent of box above the baseline.
    pub ascent: T,
    /// Maximum extent of box above the baseline.
    pub descent: T,
}
impl Extents<i32> {
    pub fn new<A, B, C>(width: A, ascent: B, descent: C) -> Self
        where A: Into<Option<i32>>,
              B: Into<Option<i32>>,
              C: Into<Option<i32>>
    {
        Extents {
            width: width.into().unwrap_or_default(),
            ascent: ascent.into().unwrap_or_default(),
            descent: descent.into().unwrap_or_default(),
        }
    }
    /// Returns the height = ascent + descent of the box
    pub fn height(&self) -> i32 {
        self.ascent + self.descent
    }
}
impl Mul<i32> for Extents<i32> {
    type Output = Extents<i32>;
    fn mul(self, _rhs: i32) -> Extents<i32> {
        Extents {
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
            width: self.width / _rhs,
            ascent: self.ascent / _rhs,
            descent: self.descent / _rhs,
        }
    }
}
impl Mul<PercentScale2D> for Extents<i32> {
    type Output = Extents<i32>;
    fn mul(self, _rhs: PercentScale2D) -> Extents<i32> {
        Extents {
            width: self.width * _rhs.horiz,
            ascent: self.ascent * _rhs.vert,
            descent: self.descent * _rhs.vert,
        }
    }
}

/// Describes the box metrics for mathematical objects.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Bounds {
    /// Position on the left on the baseline.
    pub origin: Point<i32>,
    /// Extents of the bounds.
    pub extents: Extents<i32>,
}
impl Bounds {
    #[allow(dead_code)]
    fn union_extents(self, other: Bounds) -> Extents<i32> {
        let max_x = max(self.origin.x + self.extents.width,
                        other.origin.x + other.extents.width);
        let min_x = min(self.origin.x, other.origin.x);
        let max_ascent = max(self.extents.ascent - self.origin.y,
                             other.extents.ascent - other.origin.y);
        let max_descent = max(self.extents.descent + self.origin.y,
                              other.extents.descent + other.origin.y);

        Extents {
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

impl Mul<PercentScale2D> for Bounds {
    type Output = Bounds;
    fn mul(self, _rhs: PercentScale2D) -> Bounds {
        Bounds {
            origin: self.origin * _rhs,
            extents: self.extents * _rhs,
        }
    }
}

#[derive(Debug)]
pub enum DrawInstruction {
    Glyph(Glyph),
    Line { vector: Point<i32>, thickness: u32 },
}

pub type Boxes<'a, T> = LazyVec<Box<Iterator<Item = MathBox<'a, T>> + 'a>, MathBox<'a, T>>;
pub type BoxesIter<'a, T> = lazy_vec::IntoIter<Box<Iterator<Item = MathBox<'a, T>> + 'a>,
                                               MathBox<'a, T>>;

#[derive(Debug)]
pub enum MathBoxContent<I, G> {
    Empty,
    Glyph(G),
    Line { vector: Point<i32>, thickness: u32 },
    Boxes(I),
}

impl<I, G> Default for MathBoxContent<I, G> {
    fn default() -> Self {
        MathBoxContent::Empty
    }
}

impl<'a, T: 'a> MathBoxContent<Boxes<'a, T>, (Glyph, &'a MathShaper)> {
    fn width(&self) -> i32 {
        match *self {
            MathBoxContent::Empty => 0,
            MathBoxContent::Glyph((ref glyph, ref shaper)) => shaper.glyph_advance(*glyph),
            MathBoxContent::Line { ref vector, .. } => vector.x,
            MathBoxContent::Boxes(ref boxes) => {
                boxes.as_slice()
                    .iter()
                    .map(|item| item.origin.x + item.width())
                    .max()
                    .unwrap_or_default()
            }
        }
    }

    fn vertical_metrics(&self) -> (i32, i32) {
        match *self {
            MathBoxContent::Empty => (0, 0),
            MathBoxContent::Glyph((ref glyph, ref shaper)) => shaper.glyph_extents(*glyph),
            MathBoxContent::Line { ref vector, .. } => {
                if vector.y.is_positive() {
                    (0, vector.y)
                } else {
                    (-vector.y, 0)
                }
            }
            MathBoxContent::Boxes(ref boxes) => {
                let slice = boxes.as_slice();
                (slice.iter().map(|item| -item.origin.y + item.ascent()).max().unwrap_or_default(),
                 slice.iter().map(|item| item.origin.y + item.descent()).max().unwrap_or_default())
            }
        }
    }

    fn italic_correction(&self) -> i32 {
        match *self {
            MathBoxContent::Empty => 0,
            MathBoxContent::Glyph((ref glyph, ref shaper)) => shaper.italic_correction(*glyph),
            MathBoxContent::Line { .. } => 0,
            MathBoxContent::Boxes(ref boxes) => {
                boxes.as_slice()
                    .last()
                    .map(|math_box| math_box.italic_correction())
                    .unwrap_or_default()
            }
        }
    }

    fn top_accent_attachment(&self) -> i32 {
        let value = match *self {
            MathBoxContent::Glyph((ref glyph, ref shaper)) => shaper.top_accent_attachment(*glyph),
            MathBoxContent::Boxes(ref boxes) if boxes.as_slice().len() == 1 => {
                boxes.as_slice()
                    .first()
                    .unwrap()
                    .top_accent_attachment()
            }
            _ => 0,
        };
        if value == 0 { self.width() / 2 } else { value }
    }
}

pub struct MathBox<'a, T> {
    content: MathBoxContent<Boxes<'a, T>, (Glyph, &'a MathShaper)>,
    pub origin: Point<i32>,
    extents: Extents<Cell<i32>>,
    italic_correction: Cell<i32>,
    top_accent_attachment: Cell<i32>,
    pub user_info: Option<T>,
}

impl<'a, T> Default for MathBox<'a, T> {
    fn default() -> Self {
        MathBox {
            content: Default::default(),
            origin: Default::default(),
            extents: Default::default(),
            italic_correction: Default::default(),
            top_accent_attachment: Default::default(),
            user_info: None,
        }
    }
}

impl<'a, T: 'a> MathBox<'a, T> {
    pub fn empty(extents: Extents<i32>) -> Self {
        let mut math_box = MathBox { content: MathBoxContent::Empty, ..Default::default() };
        math_box.set_extents(extents);
        math_box
    }

    pub fn with_line(from: Point<i32>, to: Point<i32>, thickness: u32) -> Self {
        MathBox {
            content: MathBoxContent::Line {
                vector: to - from,
                thickness: thickness,
            },
            origin: from,
            ..Default::default()
        }
    }

    pub fn with_glyph(glyph: Glyph, shaper: &'a MathShaper) -> Self {
        MathBox { content: MathBoxContent::Glyph((glyph, shaper)), ..Default::default() }
    }

    pub fn with_vec(vec: Vec<MathBox<'a, T>>) -> MathBox<'a, T> {
        MathBox { content: MathBoxContent::Boxes(Boxes::with_vec(vec)), ..Default::default() }
    }

    pub fn with_iter(iter: Box<Iterator<Item = MathBox<'a, T>> + 'a>) -> MathBox<'a, T> {
        MathBox { content: MathBoxContent::Boxes(Boxes::with_iter(iter)), ..Default::default() }
    }

    pub fn into_content(self) -> MathBoxContent<BoxesIter<'a, T>, Glyph> {
        match self.content {
            MathBoxContent::Empty => MathBoxContent::Empty,
            MathBoxContent::Glyph((glyph, _)) => MathBoxContent::Glyph(glyph),
            MathBoxContent::Boxes(boxes) => MathBoxContent::Boxes(boxes.into_iter()),
            MathBoxContent::Line { vector, thickness } => {
                MathBoxContent::Line {
                    vector: vector,
                    thickness: thickness,
                }
            }
        }
    }

    pub fn content(&self) -> MathBoxContent<&[MathBox<'a, T>], Glyph> {
        match self.content {
            MathBoxContent::Empty => MathBoxContent::Empty,
            MathBoxContent::Glyph((glyph, _)) => MathBoxContent::Glyph(glyph),
            MathBoxContent::Boxes(ref boxes) => MathBoxContent::Boxes(boxes.as_slice()),
            MathBoxContent::Line { vector, thickness } => {
                MathBoxContent::Line {
                    vector: vector,
                    thickness: thickness,
                }
            }
        }
    }

    pub fn width(&self) -> i32 {
        if self.extents.width.get() == 0 {
            self.extents.width.set(self.content.width());
        }
        self.extents.width.get()
    }

    fn cache_vertical_metrics(&self) {
        let (ascent, descent) = self.content.vertical_metrics();
        self.extents.ascent.set(ascent);
        self.extents.descent.set(descent);
    }

    pub fn ascent(&self) -> i32 {
        if self.extents.ascent.get() == 0 {
            self.cache_vertical_metrics();
        }
        self.extents.ascent.get()
    }

    pub fn descent(&self) -> i32 {
        if self.extents.descent.get() == 0 {
            self.cache_vertical_metrics();
        }
        self.extents.descent.get()
    }

    pub fn height(&self) -> i32 {
        self.ascent() + self.descent()
    }

    pub fn italic_correction(&self) -> i32 {
        if self.italic_correction.get() == 0 {
            self.italic_correction.set(self.content.italic_correction());
        }
        self.italic_correction.get()
    }

    pub fn top_accent_attachment(&self) -> i32 {
        if self.top_accent_attachment.get() == 0 {
            self.top_accent_attachment.set(self.content.top_accent_attachment());
        }
        self.top_accent_attachment.get()
    }

    pub fn bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: Extents {
                width: self.width(),
                ascent: self.ascent(),
                descent: self.descent(),
            },
        }
    }

    pub fn set_extents(&mut self, extents: Extents<i32>) {
        self.extents.width.set(extents.width);
        self.extents.ascent.set(extents.ascent);
        self.extents.descent.set(extents.descent);
    }

    pub fn first_glyph(&self) -> Option<Glyph> {
        match self.content {
            MathBoxContent::Glyph((glyph, _)) => Some(glyph),
            MathBoxContent::Boxes(ref boxes) => {
                boxes.as_slice().first().and_then(|math_box| math_box.first_glyph())
            }
            _ => None,
        }
    }

    pub fn last_glyph(&self) -> Option<Glyph> {
        match self.content {
            MathBoxContent::Glyph((glyph, _)) => Some(glyph),
            MathBoxContent::Boxes(ref boxes) => {
                boxes.as_slice().last().and_then(|math_box| math_box.last_glyph())
            }
            _ => None,
        }
    }
}

impl<'a, T: 'a> std::fmt::Debug for MathBox<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let extents = Extents {
            width: self.extents.width.get(),
            ascent: self.extents.ascent.get(),
            descent: self.extents.descent.get(),
        };
        f.debug_struct("MathBox")
            .field("origin", &self.origin)
            .field("extents", &extents)
            .field("italic_correction", &self.italic_correction.get())
            .field("top_accent_attachment", &self.top_accent_attachment.get())
            .field("content", &self.content())
            .finish()
    }
}
