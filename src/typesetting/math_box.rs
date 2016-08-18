use std::iter::FromIterator;
use std::cmp::{max, min};
use std::ops::{Mul, Div};
use types::Glyph;

type Boxes = Vec<MathBox>;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Point {
    pub x: i32,
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Extents {
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
}
impl Extents {
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

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Bounds {
    pub origin: Point,
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
    pub fn normalize(self) -> Bounds {
        let mut result = self;
        if self.extents.descent < 0 {
            result.origin.y += self.extents.descent;
            result.extents.descent = -self.extents.descent;
            result.extents.ascent -= -self.extents.descent;
        }
        result
    }
}

/// possible content types a MathBox can have.
#[derive(Debug, Clone)]
pub enum Content {
    Empty, // empty space e.g. like kerning
    Filled, // for fraction bars and such
    Glyph(Glyph), // a single glyph
    Boxes(Boxes), // a sublist of boxes
}
impl Default for Content {
    fn default() -> Content {
        Content::Empty
    }
}

#[derive(Debug, Default, Clone)]
pub struct MathBox {
    pub origin: Point,
    pub ink_extents: Extents,
    pub logical_extents: Extents,
    pub italic_correction: i32,
    pub top_accent_attachment: i32,
    pub content: Content,
}
impl MathBox {
    pub fn get_ink_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.ink_extents,
        }
    }
    pub fn get_logical_bounds(&self) -> Bounds {
        Bounds {
            origin: self.origin,
            extents: self.logical_extents,
        }
    }
}

impl FromIterator<MathBox> for MathBox {
    fn from_iter<I: IntoIterator<Item = MathBox>>(iter: I) -> Self {
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
