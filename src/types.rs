use std::mem;

pub type Codepoint = u64;

pub type List = Vec<ListItem>;

#[derive(Debug)]
pub enum ListItem {
    Atom(Atom),
    Hbox(MathBox),
    Vbox(MathBox)
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
pub enum AtomContents {
    Fields {
        nucleus: Field,
        top_left: Option<Field>,
        top_right: Option<Field>,
        bottom_left: Option<Field>,
        bottom_right: Option<Field>,
    },
    Translation(Field),
}
impl Default for AtomContents {
    fn default() -> AtomContents {
        AtomContents::Fields{nucleus: Default::default(), top_left: Default::default(), top_right: Default::default(),
                          bottom_left: Default::default(), bottom_right: Default::default()}
    }
}

#[derive(Debug, Default)]
pub struct Atom {
    pub atom_type: AtomType,
    pub inner: AtomContents,
}
impl Atom {
    pub fn new_with_nucleus(t: AtomType, nucleus: Field) -> Atom {
        Atom{atom_type: t, inner: AtomContents::Fields{
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
pub struct MathBox {
    pub width: i32,
    pub height: i32,
    pub bearing_x: i32,
    pub bearing_y: i32,

    pub field: Field,
}
impl MathBox {
    pub fn depth(self) -> i32 {
        self.height - self.bearing_y
    }
}

#[derive(Debug, Default)]
pub struct GeneralizedFraction {
    pub numerator: Field,
    pub denominator: Field,
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
