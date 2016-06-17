use std::mem;

pub type Codepoint = u64;

pub type List<'a> = Vec<ListItem<'a>>;

#[derive(Debug)]
pub enum ListItem<'a> {
    Atom(Atom<'a>),
    Hbox(MathBox<'a>),
    Vbox(MathBox<'a>)
}

#[derive(Debug)]
pub enum Field<'a> {
    Empty,
    Unicode(String),
    Glyph(Glyph),
    List(&'a List<'a>),
}
impl<'a> Default for Field<'a> {
    fn default() -> Field<'a> {
        Field::Empty
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum AtomType {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
    Over,
    Under,
    Acc,
    Rad,
    Vcent,
}
impl Default for AtomType {
    fn default() -> AtomType {
        AtomType::Ord
    }
}

#[derive(Debug)]
pub enum InnerAtom<'a> {
    Fields {
        nucleus: Field<'a>,
        top_left: Option<Field<'a>>,
        top_right: Option<Field<'a>>,
        bottom_left: Option<Field<'a>>,
        bottom_right: Option<Field<'a>>,
    },
    Translation(Field<'a>),
}
impl<'a> Default for InnerAtom<'a> {
    fn default() -> InnerAtom<'a> {
        InnerAtom::Fields{nucleus: Default::default(), top_left: Default::default(), top_right: Default::default(),
                          bottom_left: Default::default(), bottom_right: Default::default()}
    }
}

#[derive(Debug, Default)]
pub struct Atom<'a> {
    pub atom_type: AtomType,
    pub inner: InnerAtom<'a>,
}
impl<'a> Atom<'a> {
    pub fn new_with_nucleus<'b>(t: AtomType, nucleus: Field<'b>) -> Atom<'b> {
        Atom{atom_type: t, inner: InnerAtom::Fields{
            nucleus: nucleus,
            top_left: None,
            top_right: None,
            bottom_left: None,
            bottom_right: None}
        }
    }
}

#[derive(Debug, Default)]
pub struct BoxSize {
    pub width: i32,
    pub height: i32,
    pub bearing_x: i32,
    pub bearing_y: i32,
}
impl BoxSize {
    pub fn depth(self) -> i32 {
        self.height - self.bearing_y
    }
}

#[derive(Debug, Default)]
pub struct MathBox<'a> {
    pub width: i32,
    pub height: i32,
    pub bearing_x: i32,
    pub bearing_y: i32,

    pub field: Field<'a>,
}
impl<'a> MathBox<'a> {
    pub fn depth(self) -> i32 {
        self.height - self.bearing_y
    }
}

#[derive(Debug, Default)]
pub struct GeneralizedFraction<'a> {
    pub numerator: Field<'a>,
    pub denominator: Field<'a>,
}

#[derive(Debug, Default)]
pub struct Glyph {
    pub glyph_code: Codepoint,
    pub scale_x: i32,
    pub scale_y: i32,
}

#[derive(Debug)]
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
        let mut style: i8 = unsafe {mem::transmute(self)};
        style -= (style + 1) % 2;
        assert!(0 < style && style <= 8);
        unsafe{mem::transmute(style)}
    }

    pub fn superscript_style(self: MathStyle) -> MathStyle {
        match self {
            MathStyle::DisplayStyle | MathStyle::TextStyle => MathStyle::ScriptStyle,
            MathStyle::DisplayStylePrime | MathStyle::TextStylePrime => MathStyle::ScriptStylePrime,
            MathStyle::ScriptStyle | MathStyle::ScriptScriptStyle => MathStyle::ScriptScriptStyle,
            MathStyle::ScriptStylePrime | MathStyle::ScriptScriptStylePrime => MathStyle::ScriptScriptStylePrime,
            _ => MathStyle::Invalid,
        }
    }

    pub fn subscript_style(self: MathStyle) -> MathStyle {
        self.superscript_style().primed_style()
    }
}
