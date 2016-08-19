use std::mem;

/// An identifier of a glyph inside a font.
pub type GlyphCode = u32;

/// A List of mathematical notation objects that can be typeset.
pub type List = Vec<ListItem>;


/// List Items are the building blocks of Lists and can represent every notation object.
#[derive(Debug)]
pub enum ListItem {
    /// An expression that consists of a base (called nucleus) and optionally of attachments at
    /// each corner (e.g. subscripts and superscripts).
    Atom(Atom),
    /// An expression that consists of a base (called nucleus) and optionally of attachments that
    /// go above or below the nucleus like e.g. accents.
    OverUnder(OverUnder),
    /// A generalized version of a fraction that can also be simply a stack of objects.
    GeneralizedFraction(GeneralizedFraction),
    /// Empty space used for visually separating adjacent elements.
    Kern(Kern),
}

/// A Field is the basic building block of mathematical subexpressions. It can be a single
/// mathematical character or an entire mathematical sublist.
///
/// Typically a client will create Unicode Fields rather than Glyph fields, as the String will
/// automatically be typesetted using complex text layout and the correct glyphs will be chosen.
/// However the client can also choose to directly insert some specific glyph at the desired
/// position.

#[derive(Debug)]
pub enum Field {
    /// Nothing. This will not show in typesetted output.
    Empty,
    /// Represents some text.
    Unicode(String),
    /// Represents a specific glyph in the current font.
    Glyph(Glyph),
    /// A subexpression.
    List(List),
}
impl Default for Field {
    /// Returns the empty field.
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
    /// Returns true if the field is an empty field.
    /// # Example
    /// ```
    /// use math_render::Field;
    ///
    /// assert!(Field::Empty.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        if let Field::Empty = *self {
            true
        } else {
            false
        }
    }
}

/// An Iterator over the non-empty fields of an atom.
pub struct AtomFieldsIterator<'a> {
    atom: &'a Atom,
    state: u8, // initial value 0
}
impl<'a> Iterator for AtomFieldsIterator<'a> {
    type Item = &'a Field;
    fn next(&mut self) -> Option<&'a Field> {
        loop {
            if self.state > 4 {
                return None;
            };
            let result = match self.state {
                0 => Some(&self.atom.nucleus),
                1 => Some(&self.atom.top_left),
                2 => Some(&self.atom.top_right),
                3 => Some(&self.atom.bottom_left),
                4 => Some(&self.atom.bottom_right),
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
                /// Returns true if the field is non-empty.
                pub fn [has_ $x](&self) -> bool {
                    !(self.$x.is_empty())
                }
            }
        )*
    };
}

/// An expression that consists of a base (called nucleus) and optionally of attachments at
/// each corner (e.g. subscripts and superscripts).
#[derive(Debug, Default)]
pub struct Atom {
    /// The base of the atom.
    pub nucleus: Field,
    /// top left attachment
    pub top_left: Field,
    /// top right attachment
    pub top_right: Field,
    /// bottom left attachment
    pub bottom_left: Field,
    /// bottom right attachment
    pub bottom_right: Field,
}
impl Atom {
    /// Constructs an atom.
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

    /// Constructs an atom with empty attachments.
    pub fn new_with_nucleus(nucleus: Field) -> Atom {
        Atom {
            nucleus: nucleus,
            ..Default::default()
        }
    }

    field_accessors!(nucleus, top_left, top_right, bottom_left, bottom_right);

    /// Returns an iterator over all non-empty fields of the Atom.
    pub fn fields_iterator(&self) -> AtomFieldsIterator {
        AtomFieldsIterator {
            atom: self,
            state: 0,
        }
    }

    /// Returns true if any of the attachments is non-empty.
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

/// A structure describing a generalized fraction.
#[derive(Debug, Default)]
pub struct GeneralizedFraction {
    /// The field above the fraction bar.
    pub numerator: Field,
    /// The field below the fraction bar.
    pub denominator: Field,
}

/// A structure describing a fixed amount of whitespace.
#[derive(Debug)]
pub struct Kern {
    pub size: i32,
}

/// A font-dependent representation of a scaled glyph.
///
/// The scaling values are in percent and range from 0 to 100. A value of 100 means no rescaling in
/// that direction.
#[derive(Debug, Default, Clone)]
pub struct Glyph {
    /// The identifier of the glyph inside the font.
    pub glyph_code: GlyphCode,
    /// The horizontal scale factor in percent.
    pub scale_x: i32,
    /// The vertical scale factor in percent.
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

/// Possible positions of multiscripts relative to the base.
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
