use std::default::Default;
use std::fmt;
use std::ops::{Mul, Div};

/// An identifier of a glyph inside a font.
pub type GlyphCode = u32;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct WithUserData<T, U> {
    pub inner: T,
    pub user_data: Option<U>,
}

impl<T: Default, U> Default for WithUserData<T, U> {
    fn default() -> WithUserData<T, U> {
        WithUserData {
            inner: T::default(),
            user_data: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Index(isize);

impl ::std::convert::From<usize> for Index {
    fn from(num: usize) -> Self {
        Index(num as isize)
    }
}

impl ::std::convert::Into<usize> for Index {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl Index {
    pub fn is_none(self) -> bool {
        self.0 < 0
    }

    pub fn is_some(self) -> bool {
        !self.is_none()
    }
}

impl Default for Index {
    fn default() -> Index {
        Index(-1)
    }
}

#[derive(Default, Debug, Clone)]
pub struct MathExpression {
    entries: Vec<MathItem>,
    pub root_index: Index,
}

impl MathExpression {
    pub fn new() -> MathExpression {
        MathExpression {
            entries: Vec::new(),
            root_index: Index(-1),
        }
    }

    pub fn add_item(&mut self, item: MathItem) -> Index {
        let index = Index(self.entries.len() as isize);
        self.entries.push(item);
        index
    }

    pub fn get_item(&self, index: Index) -> Option<&MathItem> {
        if index.is_none() {
            return None;
        }
        self.entries.get(index.0 as usize)
    }

    pub fn get_item_mut(&mut self, index: Index) -> Option<&mut MathItem> {
        if index.is_none() {
            return None;
        }
        self.entries.get_mut(index.0 as usize)
    }
}

impl ::std::ops::Index<Index> for MathExpression {
    type Output = MathItem;

    fn index(&self, index: Index) -> &MathItem {
        self.get_item(index).expect("Invalid Index for MathExpression.")
    }
}

impl ::std::ops::IndexMut<Index> for MathExpression {
    fn index_mut(&mut self, index: Index) -> &mut MathItem {
        self.get_item_mut(index).expect("Invalid Index for MathExpression.")
    }
}

/// A `MathItem` is the abstract representation of mathematical notation that manages the layout
/// of its subexpressions.
#[derive(Debug, Clone, PartialEq)]
pub enum MathItem {
    /// A simple element displaying a single field without special formatting.
    Field(Field),
    /// A fixed amount of whitespace in the formula. `width` specifies the horizontal space,
    /// `ascent` the space above the baseline and `descent` the space below the baseline.
    Space(MathSpace),
    /// An expression that consists of a base (called nucleus) and optionally of attachments at
    /// each corner (e.g. subscripts and superscripts).
    Atom(Atom),
    /// An expression that consists of a base and optionally of attachments that go above or below
    /// the nucleus like e.g. accents.
    OverUnder(OverUnder),
    /// A generalized version of a fraction that can ether render as a standard fraction or
    /// as a stack of objects (e.g. for layout of mathematical vectors).
    GeneralizedFraction(GeneralizedFraction),
    /// A expression inside a radical symbol with an optional degree.
    Root(Root),
    /// A symbol that can grow horizontally or vertically to match the size of its surrounding
    /// elements.
    Operator(Operator),
    /// A list of math items to be laid out sequentially.
    List(Vec<Index>),
}

impl Default for MathItem {
    fn default() -> MathItem {
        MathItem::Field(Field::Empty)
    }
}

/// A Field is the basic building block of mathematical notation. If a `MathExpression` is
/// considered as a tree data structure, then a `Field` represents a leaf.
///
/// You can choose to create fields directly using the font-specific glyph code of the glyph to be
/// displayed or just create one from just a `String`. Typically you should create Unicode Fields
/// rather than Glyph fields, as the String will automatically be typeset using complex text
/// layout and the correct glyphs will be chosen. However if you are absolutely sure that you want
/// a certain glyph to appear in the output, This can be specified with a Glyph field.
///
/// There is also a third option to create an empty field. This should be used if for some reason
/// you don't actually want to draw anything but still get an empty 'marker'-box in the output.
/// This can be used e.g. to denote the cursor position in an equation editor.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Field {
    /// Nothing. This will not show in typeset output.
    Empty,
    /// Represents some text that should be laid out using complex text layout features of
    /// OpenType.
    Unicode(String),
    /// Represents a specific glyph in the current font.
    Glyph(Glyph),
}
impl Default for Field {
    /// Returns the empty field.
    fn default() -> Field {
        Field::Empty
    }
}
impl Field {
    /// Returns true if the field is an empty field.
    /// # Example
    /// ```
    /// use math_render::Field;
    ///
    /// assert!(Field::Empty.is_empty());
    /// assert!(!Field::Unicode("Not empty".into()).is_empty())
    /// ```
    pub fn is_empty(&self) -> bool {
        *self == Field::Empty
    }

    pub fn into_option(self) -> Option<Field> {
        match self {
            Field::Empty => None,
            _ => Some(self),
        }
    }
}

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct MathSpace {
    pub width: Length,
    pub ascent: Length,
    pub descent: Length,
}

impl MathSpace {
    pub fn horizontal_space(width: Length) -> Self {
        MathSpace {
            width: width,
            ..Default::default()
        }
    }
}

/// An expression that consists of a base (called nucleus) and attachments at each corner (e.g.
/// subscripts and superscripts).
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Atom {
    /// The base of the atom.
    pub nucleus: Index,
    /// top left attachment
    pub top_left: Index,
    /// top right attachment
    pub top_right: Index,
    /// bottom left attachment
    pub bottom_left: Index,
    /// bottom right attachment
    pub bottom_right: Index,
}


/// An expression that consists of a base (called nucleus) and attachments that go above or below
/// the nucleus like e.g. accents.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct OverUnder {
    /// the base
    pub nucleus: Index,
    /// the `Element` to go above the base
    pub over: Index,
    /// the `Element` to go below the base
    pub under: Index,
    /// the `over` element should be rendered as an accent
    pub over_is_accent: bool,
    /// the `under` element should be rendered as an accent
    pub under_is_accent: bool,
    /// If set to true the layout will not change when the current math style is `DisplayStyle` but
    /// when the current math style is `TextStyle` the `OverUnder` will be rendered as an `Atom`
    /// where the over is mapped to the top_right and the under is mapped to the bottom_right in
    /// left to right contexts.
    ///
    /// The main use of this is to display limits on large operators.
    pub is_limits: bool,
}

/// A structure describing a generalized fraction.
///
/// This can either be rendered as a fraction (with a line separating the numerator and the
/// denominator) or as a stack with no separating line (setting the `thickness`-parameter to a
/// value of 0).
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct GeneralizedFraction {
    /// The field above the fraction bar.
    pub numerator: Index,
    /// The field below the fraction bar.
    pub denominator: Index,
    /// Thickness of the fraction line. If this is zero the fraction is drawn as a stack. If
    /// thickness is None the default fraction thickness is used.
    pub thickness: Option<Length>,
}

/// An expression consisting of a radical symbol encapsulating the radicand and an optional degree
/// expression that is displayed above the beginning of the surd.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Root {
    /// The expression "inside" of the radical symbol.
    pub radicand: Index,
    /// The degree of the radical.
    pub degree: Index,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct StretchConstraints {
    pub min_size: Option<Length>,
    pub max_size: Option<Length>,
    pub symmetric: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Operator {
    pub stretch_constraints: Option<StretchConstraints>,
    pub is_large_op: bool,
    pub leading_space: Length,
    pub trailing_space: Length,
    pub field: Field,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LengthUnit {
    /// A point traditionally equals 1/72 of an inch.
    Point,
    /// Current EM-Size.
    Em,
    /// The minimum height to display a display operator.
    DisplayOperatorMinHeight,
}

/// Lengths are specified with a numeric value an a unit.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Length {
    pub value: f32,
    pub unit: LengthUnit,
}

impl Length {
    pub fn new(val: f32, unit: LengthUnit) -> Self {
        Length {
            value: val,
            unit: unit,
        }
    }

    pub fn is_null(self) -> bool {
        self.value == 0.0
    }

    pub fn em(val: f32) -> Self {
        Length::new(val, LengthUnit::Em)
    }
}

impl Default for Length {
    fn default() -> Length {
        Length {
            value: 0.0,
            unit: LengthUnit::Point,
        }
    }
}

/// A type for representing fractional scale values in percent. A value of 100 means original size,
/// 50 means scaled to half the original size.
///
/// # Examples
/// ```
/// # use math_render::PercentValue;
/// let scale = PercentValue::new(50);
/// let num = 300;
/// assert_eq!(150, num * scale);
/// ```
#[derive(Default, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PercentValue {
    percent: u8,
}

impl PercentValue {
    /// Create a new `PercentValue` from an integer between 0 and 100 representing the percentage.
    pub fn new(value: u8) -> PercentValue {
        debug_assert!(value <= 100, "Not a valid percent value");
        // for release builds still make sure that percentage is valid
        let value = if value > 100 { 100u8 } else { value };
        PercentValue { percent: value }
    }

    /// Returns the percentage as an unsigned integer.
    ///
    /// # Examples
    /// ```
    /// # use math_render::PercentValue;
    /// let percent = PercentValue::new(64);
    /// assert_eq!( 64, percent.as_percentage() );
    /// ```
    pub fn as_percentage(self) -> u8 {
        self.percent
    }

    /// Returns the scale factor corresponding to the percentage. Essentially the percentage
    /// divided by 100 represented as a floating point number.
    ///
    /// # Examples
    /// ```
    /// # use math_render::PercentValue;
    /// let percent = PercentValue::new(50);
    /// assert_eq!( 0.5f32, percent.as_scale_mult() );
    /// ```
    pub fn as_scale_mult(self) -> f32 {
        (self.percent as f32) / 100f32
    }
}

impl fmt::Debug for PercentValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} %", self.percent)
    }
}


impl Mul<i32> for PercentValue {
    type Output = i32;

    fn mul(self, _rhs: i32) -> i32 {
        let value = _rhs.saturating_mul(self.percent as i32);
        value / 100i32
    }
}

impl Mul<PercentValue> for i32 {
    type Output = i32;

    fn mul(self, _rhs: PercentValue) -> i32 {
        _rhs * self
    }
}

impl Div<PercentValue> for i32 {
    type Output = i32;

    fn div(self, _rhs: PercentValue) -> i32 {
        if _rhs.percent == 100 {
            self
        } else {
            let value = self * 100i32;
            value / (_rhs.percent as i32)
        }
    }
}

impl Div<PercentValue> for u32 {
    type Output = u32;

    fn div(self, _rhs: PercentValue) -> u32 {
        if _rhs.percent == 100 {
            self
        } else {
            let value = self * 100u32;
            value / (_rhs.percent as u32)
        }
    }
}

/// A font-dependent representation of a (possibly scaled) glyph.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Glyph {
    /// The identifier of the glyph inside the font.
    pub glyph_code: GlyphCode,

    /// The scaling to apply to this glyph
    pub scale: PercentValue,
}

/// Vertical layout style for equations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MathStyle {
    /// Style for equations that are displayed in their own line.
    Display,
    /// Style for equations to be displayed inline with text.
    Inline,
}

/// Determines the general style how a math expression should be laid out.
///
/// This affects lots of parameters when laying out an equation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutStyle {
    /// This affects how much vertical space the equation will use.
    pub math_style: MathStyle,
    /// When the `script_level` property is non-null the glyphs of the font are scaled down. To be
    /// used e.g. when rendering subscripts.
    pub script_level: u8,
    /// If `true` superscripts and similar protrude less at the top.
    pub is_cramped: bool,
}

impl LayoutStyle {
    /// Returns a new `LayoutStyle` with default settings
    pub fn new() -> LayoutStyle {
        Default::default()
    }

    pub fn inline_style(self) -> Self {
        LayoutStyle {
            math_style: MathStyle::Inline,
            ..self
        }
    }

    pub fn display_style(self) -> Self {
        LayoutStyle {
            math_style: MathStyle::Display,
            ..self
        }
    }

    pub fn with_increased_script_level(self) -> Self {
        LayoutStyle {
            script_level: self.script_level.saturating_add(1),
            ..self
        }
    }

    pub fn with_decreased_script_level(self) -> Self {
        LayoutStyle {
            script_level: self.script_level.saturating_sub(1),
            ..self
        }
    }

    /// Returns a cramped version of the style.
    ///
    /// If the style is already cramped it is left unaltered. Cramped styles try to limit vertical
    /// extent of equations above the text. This is used for example in denominators of fractions or
    /// subscripts and similar.
    pub fn cramped_style(self) -> LayoutStyle {
        LayoutStyle {
            is_cramped: true,
            ..self
        }
    }
    }

    /// Returns the style that the superscript of a base styled with `self` should have.
    pub fn superscript_style(self) -> LayoutStyle {
        LayoutStyle {
            math_style: MathStyle::Inline,
            script_level: self.script_level + 1,
            ..self
        }
    }

    /// Returns the style that the subscript of a base styled with `self` should have.
    pub fn subscript_style(self) -> LayoutStyle {
        self.superscript_style().cramped_style()
    }
}

impl Default for LayoutStyle {
    fn default() -> LayoutStyle {
        LayoutStyle {
            math_style: MathStyle::Display,
            script_level: 0,
            is_cramped: false,
        }
    }
}

/// Possible positions of multiscripts relative to the base.
#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum CornerPosition {
    /// Prescript top
    TopLeft = 1,
    /// Superscript position
    TopRight = 0,
    /// Prescript bottom
    BottomLeft = 3,
    /// Subscript position
    BottomRight = 2,
}


pub use self::CornerPosition::{TopLeft, TopRight, BottomLeft, BottomRight};
impl CornerPosition {
    /// Returns true if the position is left of the base
    pub fn is_left(self) -> bool {
        match self {
            TopLeft | BottomLeft => true,
            _ => false,
        }
    }

    /// Returns true if the position is right of the base
    pub fn is_top(self) -> bool {
        match self {
            TopLeft | TopRight => true,
            _ => false,
        }
    }

    /// Returns the position that is horizontally "on the other side".
    pub fn horizontal_mirror(self) -> Self {
        match self {
            TopLeft => TopRight,
            TopRight => TopLeft,
            BottomLeft => BottomRight,
            BottomRight => BottomLeft,
        }
    }

    /// Returns the position that is vertically opposite.
    pub fn vertical_mirror(self) -> Self {
        match self {
            TopLeft => BottomLeft,
            TopRight => BottomRight,
            BottomLeft => TopLeft,
            BottomRight => TopRight,
        }
    }

    /// Returns the position that is both horizontally and vertically mirrored.
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
    use super::*;

    #[test]
    #[should_panic(expected = "Not a valid percent value")]
    fn percent_test() {
        let val = PercentValue::new(101);
        assert_eq!(val.as_percentage(), 101);
    }

    #[test]
    fn test_math_expression() {
        let mut graph = MathExpression::new();
        let first_node = MathItem::default();
        let second_node = MathItem::default();
        let third_node = MathItem::default();

        let first_index = graph.add_item(first_node);
        let second_index = graph.add_item(second_node);
        let third_index = graph.add_item(third_node);

        assert_eq!(first_index, Index(0));
        assert_eq!(second_index, Index(1));
        assert_eq!(third_index, Index(2));
    }
}
