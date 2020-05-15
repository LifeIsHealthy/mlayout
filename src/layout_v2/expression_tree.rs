// Copyright 2018 Manuel Reinhardt
// 
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// 
//     http://www.apache.org/licenses/LICENSE-2.0
// 
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use generational_arena::{Arena, Index};

use super::field::Field;

use super::layout::{Layout, LayoutRecursive, Context, Request, Response};

use crate::math_box::MathBox;

/// A `MathItem` is the abstract representation of mathematical notation that manages the layout
/// of its subexpressions.
#[derive(Debug)]
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
}

impl LayoutRecursive for MathItem {
    fn layout_recursive(&self, callback: impl FnOnce(&dyn Layout<Return=Option<MathBox>>) -> Option<MathBox>) -> Option<MathBox> {
        unimplemented!()
    }
}

impl Default for MathItem {
    fn default() -> MathItem {
        MathItem::Field(Field::Empty)
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
#[derive(Default, Debug)]
pub struct Atom {
    /// The base of the atom.
    pub nucleus: Option<NodeRef>,
    /// top left attachment
    pub top_left: Option<NodeRef>,
    /// top right attachment
    pub top_right: Option<NodeRef>,
    /// bottom left attachment
    pub bottom_left: Option<NodeRef>,
    /// bottom right attachment
    pub bottom_right: Option<NodeRef>,
}

/// An expression that consists of a base (called nucleus) and attachments that go above or below
/// the nucleus like e.g. accents.
#[derive(Debug, Default)]
pub struct OverUnder {
    /// the base
    pub nucleus: Option<NodeRef>,
    /// the `Element` to go above the base
    pub over: Option<NodeRef>,
    /// the `Element` to go below the base
    pub under: Option<NodeRef>,
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
#[derive(Debug, Default)]
pub struct GeneralizedFraction {
    /// The field above the fraction bar.
    pub numerator: Option<NodeRef>,
    /// The field below the fraction bar.
    pub denominator: Option<NodeRef>,
    /// Thickness of the fraction line. If this is zero the fraction is drawn as a stack. If
    /// thickness is None the default fraction thickness is used.
    pub thickness: Option<NodeRef>,
}

/// An expression consisting of a radical symbol encapsulating the radicand and an optional degree
/// expression that is displayed above the beginning of the surd.
#[derive(Debug, Default)]
pub struct Root {
    /// The expression "inside" of the radical symbol.
    pub radicand: Option<NodeRef>,
    /// The degree of the radical.
    pub degree: Option<NodeRef>,
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

pub struct ExpressionTree {
    nodes: Arena<Node>
}

impl ExpressionTree {
    fn layout(&mut self, node: NodeRef) -> Option<MathBox> {
        let node = self.nodes.remove(node.0).unwrap();

        let math_box = node.item.layout_recursive(|layout_iter| {
            let mut argument = None;
            loop {
                match layout_iter.step(Context::default(), argument) {
                    Request::Node(node, constraints) => {
                        argument = Some(Response::Layout(self.layout(node).unwrap()));
                    },
                    _ => {}
                }
            }
        });

        None
    }
}

pub struct Node {
    item: MathItem,
    next: Option<NodeRef>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeRef(Index);