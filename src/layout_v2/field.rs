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

use super::layout::{LayoutRecursive, Layout};

use crate::math_box::MathBox;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Field {
    /// Nothing. This will not show in typeset output.
    Empty,
    /// Represents some text that should be laid out using complex text layout features of
    /// OpenType.
    Unicode(String),
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

impl LayoutRecursive for Field {
    fn layout_recursive(&self, callback: impl FnOnce(&dyn Layout<Return=Option<MathBox>>) -> Option<MathBox>) -> Option<MathBox> {
        unimplemented!()
    }
}
