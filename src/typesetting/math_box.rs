use std::iter::FromIterator;
use std::cmp::{max, min};
use std::ops::{Mul, Div};
use types::Glyph;

type Boxes = Vec<MathBox>;

/// A point in 2D space.
///
/// Note: The y coordinate increases downwards.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Point {
    /// the x coordinate
    pub x: i32,
    /// the y coordinate
    pub y: i32,
}
impl Mul<i32> for Point {
    type Output = Point;
    fn mul(self, _rhs: i32) -> Point {
        Point {
            x: self.x * _rhs,
            y: self.y * _rhs,
        }
    }
}
impl Div<i32> for Point {
    type Output = Point;
    fn div(self, _rhs: i32) -> Point {
        Point {
            x: self.x / _rhs,
            y: self.y / _rhs,
        }
    }
}

/// Basic Extents of boxes
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Extents {
    /// Width of the box
    pub width: i32,
    /// Maximum extent of box above the baseline.
    pub ascent: i32,
    /// Maximum extent of box above the baseline.
    pub descent: i32,
}
impl Extents {
    /// Returns the height = ascent + descent of the box
    pub fn height(&self) -> i32 {
        self.ascent + self.descent
    }
}
impl Mul<i32> for Extents {
    type Output = Extents;
    fn mul(self, _rhs: i32) -> Extents {
        Extents {
            width: self.width * _rhs,
            ascent: self.ascent * _rhs,
            descent: self.descent * _rhs,
        }
    }
}
impl Div<i32> for Extents {
    type Output = Extents;
    fn div(self, _rhs: i32) -> Extents {
        Extents {
            width: self.width / _rhs,
            ascent: self.ascent / _rhs,
            descent: self.descent / _rhs,
        }
    }
}

/// Describes the box metrics for mathematical objects.
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Bounds {
    /// Position on the left on the baseline.
    pub origin: Point,
    /// Extents of the bounds.
    pub extents: Extents,
}
impl Bounds {
    fn union_extents(self, other: Bounds) -> Extents {
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
    /// Returns bounds that have non negative ascent and descent by moving the origin.
    pub fn normalize(self) -> Bounds {
        let mut result = self;
        if result.extents.descent < 0 {
            result.origin.y += result.extents.descent;
            result.extents.descent = -result.extents.descent;
            result.extents.ascent -= result.extents.descent;
        }
        if result.extents.ascent < 0 {
            result.origin.y -= result.extents.ascent;
            result.extents.ascent = -result.extents.ascent;
            result.extents.descent -= result.extents.ascent;
        }
        result
    }
}

/// Possible content types a MathBox can have.
#[derive(Debug, Clone)]
pub enum Content {
    /// empty space e.g. like kerning
    Empty,
    /// for fraction bars and such
    Filled,
    ///  a single glyph
    Glyph(Glyph),
    /// a sublist of boxes
    Boxes(Boxes),
}
impl Default for Content {
    fn default() -> Content {
        Content::Empty
    }
}

/// A box that contains all the metrics of a mathematical subexpression.
///
/// It has two
///
/// See also: [MathML in HTML5 - Implementation Note](http://mathml-association.org/MathMLinHTML5/S3.html#SS1.SSS1)
#[derive(Debug, Default, Clone)]
pub struct MathBox {
    /// The logical position of the Box on the baseline.
    pub origin: Point,
    /// The extents of the ink inside the box.
    pub ink_extents: Extents,
    /// Logical extents that may show designer's intent or additional free space neede around the
    /// box.
    pub logical_extents: Extents,
    /// The italic correction of the entire box.
    pub italic_correction: i32,
    /// A value controlling the placement of top attachments.
    pub top_accent_attachment: i32,
    /// The content of the box.
    pub content: Content,
}
impl MathBox {
    /// Returns bounds with the `ink_extents` as their extents and the box origin as their origin.
    ///
    /// # Example
    /// ```
    /// use math_render::math_box::{Bounds, MathBox, Point, Extents};
    ///
    /// let bounds = Bounds { origin: Point { x: 10, y: 20 },
    ///                       extents: Extents { width: 30, ascent: 40, descent: 50 } };
    /// let math_box = MathBox { origin: bounds.origin, ink_extents: bounds.extents, ..Default::default() };
    /// assert_eq!(bounds, math_box.get_ink_bounds())
    /// ```
    pub fn get_ink_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.ink_extents,
        }
    }
    /// Returns bounds with the `logical_extents` as their extents and the box origin as their origin.
    ///
    /// # Example
    /// ```
    /// use math_render::math_box::{Bounds, MathBox, Point, Extents};
    ///
    /// let bounds = Bounds { origin: Point { x: 10, y: 20 },
    ///                       extents: Extents { width: 30, ascent: 40, descent: 50 } };
    /// let math_box = MathBox { origin: bounds.origin, logical_extents: bounds.extents, ..Default::default() };
    /// assert_eq!(bounds, math_box.get_logical_bounds())
    /// ```
    pub fn get_logical_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.logical_extents,
        }
    }
}

impl FromIterator<MathBox> for MathBox {
    fn from_iter<I: IntoIterator<Item=MathBox>>(iter: I) -> Self {
        let mut result = MathBox { content: Content::Boxes(Boxes::new()), ..Default::default() };
        let iter = iter.into_iter();
        let mut count = 0;
        let mut top_accent_attachment = 0;

        iter.fold(&mut result, |mut acc, math_box| {
            {

                if count == 0 {
                    top_accent_attachment = math_box.top_accent_attachment;
                    acc.logical_extents = math_box.logical_extents;
                    acc.ink_extents = math_box.ink_extents;
                } else {
                    acc.logical_extents = acc.get_logical_bounds()
                        .union_extents(math_box.get_logical_bounds());
                    acc.ink_extents = acc.get_ink_bounds().union_extents(math_box.get_ink_bounds());
                }
                acc.italic_correction = math_box.italic_correction;
                count += 1;

                let &mut MathBox { ref mut content, .. } = acc;
                if let Content::Boxes(ref mut list) = *content {
                    list.push(math_box);
                } else {
                    unreachable!();
                };
            }
            acc
        });
        if count == 1 {
            result.top_accent_attachment = top_accent_attachment;
        } else {
            result.top_accent_attachment = result.logical_extents.width / 2;
        }
        result
    }
}
