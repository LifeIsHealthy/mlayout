use std::mem;

pub type Codepoint = u64;
pub type GlyphCode = u32;

pub type List = Vec<ListItem>;

#[derive(Debug)]
pub enum ListItem {
    Atom(Atom),
    OverUnder(OverUnder),
    GeneralizedFraction(GeneralizedFraction),
    Kern(Kern),
}

#[derive(Debug)]
pub enum Field {
    Empty,
    Unicode(String),
    Glyph(Glyph),
    List(List),
}
impl Default for Field {
    fn default() -> Field {
        Field::Empty
    }
}
impl ::std::convert::From<ListItem> for Field {
    fn from(item: ListItem) -> Field {
        Field::List(vec![item])
    }
}
impl Field {
    pub fn is_empty(&self) -> bool {
        if let Field::Empty = *self {
            true
        } else {
            false
        }
    }
}

// initialize with state = 0
pub struct AtomFieldsIterator<'a> {
    atom: &'a Atom,
    state: u8,
}
impl<'a> Iterator for AtomFieldsIterator<'a> {
    type Item = &'a Field;
    fn next(&mut self) -> Option<&'a Field> {
        loop {
            if self.state > 4 {
                return None;
            };
            let result = match self.state {
                0 => Some(self.atom.nucleus_ref()),
                1 => Some(self.atom.top_left_ref()),
                2 => Some(self.atom.top_right_ref()),
                3 => Some(self.atom.bottom_left_ref()),
                4 => Some(self.atom.bottom_right_ref()),
                _ => None,
            };
            self.state += 1;
            match result {
                Some(field) if !field.is_empty() => return Some(field),
                _ => {},
            };
        }
    }
}

macro_rules! field_accessors {
    ( $( $x:ident ),* ) => {
        $(
            interpolate_idents! {
                pub fn [has_ $x](&self) -> bool {
                    !(self.$x.is_empty())
                }

                pub fn [$x _ref](&self) -> &Field {
                    &self.$x
                }

                pub fn [$x _ref_mut](&mut self) -> &mut Field {
                    &mut self.$x
                }

                pub fn $x(self) -> Field {
                    self.$x
                }
            }
        )*
    };
}

#[derive(Debug, Default)]
pub struct Atom {
    pub nucleus: Field,
    pub top_left: Field,
    pub top_right: Field,
    pub bottom_left: Field,
    pub bottom_right: Field,
}
impl Atom {
    pub fn new_with_attachments(
               nucleus: Field,
               top_left: Field,
               top_right: Field,
               bottom_left: Field,
               bottom_right: Field)
               -> Atom {
        Atom {
            nucleus: nucleus,
            top_left: top_left,
            top_right: top_right,
            bottom_left: bottom_left,
            bottom_right: bottom_right,
            ..Default::default()
        }
    }

    pub fn new_with_nucleus(nucleus: Field) -> Atom {
        Atom {
            nucleus: nucleus,
            ..Default::default()
        }
    }

    field_accessors!(nucleus, top_left, top_right, bottom_left, bottom_right);

    pub fn fields_iterator(&self) -> AtomFieldsIterator {
        AtomFieldsIterator {
            atom: self,
            state: 0,
        }
    }
    pub fn has_any_attachments(&self) -> bool {
        self.has_top_left() || self.has_top_right() || self.has_bottom_left() ||
        self.has_bottom_right()
    }
}

#[derive(Debug, Default)]
pub struct OverUnder {
    pub nucleus: Field,
    pub over: Field,
    pub under: Field,
    pub over_is_accent: bool,
    pub under_is_accent: bool,
}
impl OverUnder {
    field_accessors!(nucleus, over, under);
}

#[derive(Debug, Default)]
pub struct GeneralizedFraction {
    pub numerator: Field,
    pub denominator: Field,
}

#[derive(Debug)]
pub struct Kern {
    pub size: i32,
}

#[derive(Debug, Default, Clone)]
pub struct Glyph {
    pub glyph_code: GlyphCode,
    pub scale_x: i32,
    pub scale_y: i32,
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq, Ord)]
#[repr(i8)]
pub enum MathStyle {
    DisplayStyle = 8,
    DisplayStylePrime = 7,
    TextStyle = 6,
    TextStylePrime = 5,
    ScriptStyle = 4,
    ScriptStylePrime = 3,
    ScriptScriptStyle = 2,
    ScriptScriptStylePrime = 1,
    Invalid = 0,
    Increase = -1,
    Decrease = -2,
}
impl MathStyle {
    pub fn primed_style(self: MathStyle) -> MathStyle {
        let mut style: i8 = unsafe { mem::transmute(self) };
        style -= (style + 1) % 2;
        assert!(0 < style && style <= 8);
        unsafe { mem::transmute(style) }
    }

    pub fn superscript_style(self: MathStyle) -> MathStyle {
        match self {
            MathStyle::DisplayStyle | MathStyle::TextStyle => MathStyle::ScriptStyle,
            MathStyle::DisplayStylePrime |
            MathStyle::TextStylePrime => MathStyle::ScriptStylePrime,
            MathStyle::ScriptStyle |
            MathStyle::ScriptScriptStyle => MathStyle::ScriptScriptStyle,
            MathStyle::ScriptStylePrime |
            MathStyle::ScriptScriptStylePrime => MathStyle::ScriptScriptStylePrime,
            _ => MathStyle::Invalid,
        }
    }

    pub fn subscript_style(self: MathStyle) -> MathStyle {
        self.superscript_style().primed_style()
    }

    pub fn is_cramped(self) -> bool {
        let style = self as i8;
        style % 2 == 1
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum CornerPosition {
    TopLeft = 1,
    TopRight = 0,
    BottomLeft = 3,
    BottomRight = 2,
}


pub use self::CornerPosition::{TopLeft, TopRight, BottomLeft, BottomRight};
impl CornerPosition {

    pub fn is_left(self) -> bool {
        match self {
            TopLeft | BottomLeft => true,
            _ => false
        }
    }
    pub fn is_top(self) -> bool {
        match self {
            TopLeft | TopRight => true,
            _ => false
        }
    }
    pub fn horizontal_mirror(self) -> Self {
        match self {
            TopLeft => TopRight,
            TopRight => TopLeft,
            BottomLeft => BottomRight,
            BottomRight => BottomLeft,
        }
    }
    pub fn vertical_mirror(self) -> Self {
        match self {
            TopLeft => BottomLeft,
            TopRight => BottomRight,
            BottomLeft => TopLeft,
            BottomRight => TopRight,
        }
    }
    pub fn diagonal_mirror(self) -> Self {
        match self {
            TopLeft => BottomRight,
            TopRight => BottomLeft,
            BottomLeft => TopRight,
            BottomRight => TopLeft,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MathStyle::*;

    #[test]
    fn math_style_test() {
        assert!(ScriptScriptStyle < ScriptStylePrime);
        assert!(DisplayStyle > DisplayStylePrime);
    }
}
