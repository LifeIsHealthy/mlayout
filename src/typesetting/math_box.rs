use std::iter::FromIterator;
use std::cmp::{max, min};
use std::ops::{Mul, Div};
use types::{Glyph, PercentScale2D};

type Boxes<T> = Vec<MathBox<T>>;

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
impl Mul<PercentScale2D> for Point {
    type Output = Point;
    fn mul(self, _rhs: PercentScale2D) -> Point {
        Point {
            x: self.x * _rhs.horiz,
            y: self.y * _rhs.vert,
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
impl Mul<PercentScale2D> for Extents {
    type Output = Extents;
    fn mul(self, _rhs: PercentScale2D) -> Extents {
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

/// Possible content types a `MathBox` can have.
#[derive(Clone)]
pub enum Content<T> {
    /// empty space e.g. like kerning
    Empty,
    /// for fraction bars and such
    Filled,
    ///  a single glyph
    Glyph(Glyph),
    /// a sublist of boxes
    Boxes(Boxes<T>),
}
impl<T> ::std::fmt::Debug for Content<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            Content::Empty => write!(f, "Empty"),
            Content::Filled => write!(f, "Empty"),
            Content::Glyph(ref glyph) => glyph.fmt(f),
            Content::Boxes(ref boxes) => boxes.fmt(f),
        }
    }
}
impl<T> Default for Content<T> {
    fn default() -> Content<T> {
        Content::Empty
    }
}

/// A box that contains all the metrics of a mathematical subexpression.
///
/// It has two
///
/// See also: [`MathML` in HTML5 - Implementation Note]
/// (http://mathml-association.org/MathMLinHTML5/S3.html#SS1.SSS1)
#[derive(Clone)]
pub struct MathBox<T> {
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
    pub content: Content<T>,
    /// User provided information.
    pub user_info: Option<T>,
}
impl<T> MathBox<T> {
    /// Returns bounds with the `ink_extents` as their extents and the box origin as their origin.
    ///
    /// # Example
    /// ```
    /// use math_render::math_box::{Bounds, MathBox, Point, Extents};
    ///
    /// let bounds = Bounds { origin: Point { x: 10, y: 20 },
    ///                       extents: Extents { width: 30, ascent: 40, descent: 50 } };
    /// let math_box: MathBox<()> = MathBox {
    ///     origin: bounds.origin,
    ///     ink_extents: bounds.extents,
    ///     ..Default::default()
    /// };
    /// assert_eq!(bounds, math_box.get_ink_bounds())
    /// ```
    pub fn get_ink_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.ink_extents,
        }
    }
    /// Returns bounds with the `logical_extents` as their extents and the box origin as their
    /// origin.
    ///
    /// # Example
    /// ```
    /// use math_render::math_box::{Bounds, MathBox, Point, Extents};
    ///
    /// let bounds = Bounds { origin: Point { x: 10, y: 20 },
    ///                       extents: Extents { width: 30, ascent: 40, descent: 50 } };
    /// let math_box: MathBox<()> = MathBox {
    ///     origin: bounds.origin,
    ///     logical_extents: bounds.extents,
    ///     ..Default::default()
    /// };
    /// assert_eq!(bounds, math_box.get_logical_bounds())
    /// ```
    pub fn get_logical_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.logical_extents,
        }
    }
}

impl<T> ::std::fmt::Debug for MathBox<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct("MathBox")
            .field("origin", &self.origin)
            .field("ink_extents", &self.ink_extents)
            .field("content", &self.content)
            .finish()
    }
}

impl<T> ::std::default::Default for MathBox<T> {
    fn default() -> MathBox<T> {
        MathBox {
            origin: Default::default(),
            ink_extents: Default::default(),
            logical_extents: Default::default(),
            italic_correction: Default::default(),
            top_accent_attachment: Default::default(),
            content: Default::default(),
            user_info: None,
        }
    }
}

impl<T> FromIterator<MathBox<T>> for MathBox<T> {
    fn from_iter<I: IntoIterator<Item = MathBox<T>>>(iter: I) -> Self {
        let mut iter = iter.into_iter().peekable();

        let mut result = match iter.next() {
            Some(item) => item,
            None => return Default::default(),
        };

        // return immediately if there is just one box in the iterator
        if iter.peek().is_none() {
            return result;
        } else {
            let mut new_result = MathBox {
                origin: Default::default(),
                content: Content::Boxes(Boxes::new()),
                user_info: None,
                ..result
            };

            // adjust the vertical extents, as the new box' coordinates start from origin
            new_result.ink_extents.ascent -= result.origin.y;
            new_result.ink_extents.descent += result.origin.y;
            new_result.logical_extents.ascent -= result.origin.y;
            new_result.logical_extents.descent += result.origin.y;

            match new_result.content {
                Content::Boxes(ref mut list) => list.push(result),
                _ => unreachable!(),
            }
            result = new_result;
        }

        for math_box in iter {
            result.logical_extents = result.get_logical_bounds()
                .union_extents(math_box.get_logical_bounds());
            result.ink_extents = result.get_ink_bounds().union_extents(math_box.get_ink_bounds());
            result.italic_correction = math_box.italic_correction;

            match result.content {
                Content::Boxes(ref mut list) => list.push(math_box),
                _ => unreachable!(),
            }
        }

        result.top_accent_attachment = result.logical_extents.width / 2;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_test() {
        let bounds = Bounds {
            origin: Point { x: 10, y: 20 },
            extents: Extents {
                width: 100,
                ascent: 200,
                descent: -100,
            },
        };
        let normalized = Bounds {
            origin: Point { x: 10, y: -80 },
            extents: Extents {
                width: 100,
                ascent: 100,
                descent: 0,
            },
        };
        assert_eq!(bounds.normalize(), normalized);

        let bounds = Bounds {
            origin: Point { x: 10, y: 20 },
            extents: Extents {
                width: 100,
                ascent: -100,
                descent: 150,
            },
        };
        let normalized = Bounds {
            origin: Point { x: 10, y: 120 },
            extents: Extents {
                width: 100,
                ascent: 0,
                descent: 50,
            },
        };
        assert_eq!(bounds.normalize(), normalized);
    }

    #[test]
    #[should_panic]
    fn invalid_bounds_test() {
        let bounds = Bounds {
            origin: Point { x: 10, y: 20 },
            extents: Extents {
                width: 100,
                ascent: 200,
                descent: -300,
            },
        };
        bounds.normalize();
    }
}
