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

use super::expression_tree::NodeRef;
use crate::math_box::MathBox;

#[derive(Default)]
pub struct Context {
    constraints: Constraints
}

#[derive(Default)]
pub struct Constraints {}

pub enum Response {
    NotFound,
    Layout(MathBox),
}

pub enum Request<Ret> {
    Return(Ret),
    Node(NodeRef, Constraints),
}

pub trait Layout {
    type Return;

    fn step(&mut self, context: Context, argument: Option<Response>) -> Request<Self::Return>;
    
    // fn may_stretch(&mut self, ...) -> Request<bool>;

    fn map<F, U, T>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnOnce(U) -> T,
    {
        Map {
            inner: self,
            map: f,
        }
    }

    fn chain<L>(self, other: L) -> Chain<Self, L>
    where
        Self: Sized,
        L: Layout,
    {
        Chain {
            left: self,
            right: other,
            left_val: None,
        }
    }
}

pub trait LayoutRecursive {
    fn layout_recursive(&self, callback: impl FnOnce(&mut dyn Layout<Return=Option<MathBox>>) -> Option<MathBox>) -> Option<MathBox>;
}

pub struct LayoutOne {
    node_ref: NodeRef,
}

impl Layout for LayoutOne {
    type Return = Option<MathBox>;

    fn step(&mut self, context: Context, argument: Option<Response>) -> Request<Self::Return> {
        match argument {
            None => Request::Node(self.node_ref, context.constraints),
            Some(Response::Layout(math_box)) => Request::Return(Some(math_box)),
            Some(Response::NotFound) => Request::Return(None),
        }
    }
}

pub struct Map<L, F> {
    inner: L,
    map: F,
}

impl<L, F, U, T> Layout for Map<L, F>
where
    L: Layout<Return = U>,
    F: FnOnce(U) -> T,
{
    type Return = T;

    fn step(&mut self, context: Context, argument: Option<Response>) -> Request<T> {
        let request = self.inner.step(context, argument);
        match request {
            Request::Return(val) => Request::Return((self.map)(val)),
            Request::Node(node_ref, constr) => Request::Node(node_ref, constr),
        }
    }
}

pub struct Chain<Left: Layout, Right: Layout> {
    left: Left,
    right: Right,
    left_val: Option<Left::Return>,
}

impl<Left, Right> Layout for Chain<Left, Right>
where
    Left: Layout,
    Right: Layout,
{
    type Return = (Left::Return, Right::Return);

    fn step(&mut self, context: Context, argument: Option<Response>) -> Request<Self::Return> {
        loop {
            match self.left_val {
                None => {
                    let request = self.left.step(context, argument);
                    match request {
                        Request::Return(left_val) => {
                            self.left_val = Some(left_val);
                        }
                        Request::Node(node_ref, constr) => {
                            break Request::Node(node_ref, constr)
                        }
                    }
                }
                Some(left_val) => {
                    let request = self.right.step(context, argument);
                    match request {
                        Request::Return(right_val) => break Request::Return((left_val, right_val)),
                        Request::Node(node_ref, constr) => {
                            break Request::Node(node_ref, constr)
                        }
                    }
                }
            }
        }
    }
}

impl<T> Layout for Option<T> {
    type Return = Option<T>;

    fn step(&mut self, context: Context, argument: Option<Response>) -> Request<Self::Return> {
        Request::Return(*self)
    }
}
