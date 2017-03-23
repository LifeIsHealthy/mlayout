use std::mem;
use std::cell::UnsafeCell;

pub enum LazyVecInner<I: Iterator> {
    Iter(I),
    Vec(Vec<I::Item>),
}

impl<I: Iterator> LazyVecInner<I> {
    fn replace_with_vec(&mut self) {
        let vec = if let LazyVecInner::Iter(ref mut iter) = *self {
            iter.collect()
        } else {
            return;
        };
        mem::replace(self, LazyVecInner::Vec(vec));
    }
}

pub struct LazyVec<I: Iterator>(UnsafeCell<LazyVecInner<I>>);

impl<I: Iterator> LazyVec<I> {
    pub fn with_iter(iter: I) -> Self {
        LazyVec(UnsafeCell::new(LazyVecInner::Iter(iter)))
    }

    pub fn with_vec(vec: Vec<I::Item>) -> Self {
        LazyVec(UnsafeCell::new(LazyVecInner::Vec(vec)))
    }

    pub fn as_slice(&self) -> &[I::Item] {
        let mut inner = unsafe { &mut *self.0.get() };
        inner.replace_with_vec();
        match *inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(ref vec) => &vec[..],
        }
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<I::Item> {
        let mut inner = unsafe { &mut *self.0.get() };
        inner.replace_with_vec();
        match *inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(ref mut vec) => vec,
        }
    }

    pub fn into_vec(self) -> Vec<I::Item> {
        let mut inner = unsafe { self.0.into_inner() };
        inner.replace_with_vec();
        match inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(vec) => vec,
        }
    }
}

pub enum IntoIter<I: Iterator> {
    Iter(I),
    VecIter(::std::vec::IntoIter<I::Item>),
}

impl<I: Iterator> Iterator for IntoIter<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        match *self {
            IntoIter::Iter(ref mut iter) => iter.next(),
            IntoIter::VecIter(ref mut iter) => iter.next(),
        }
    }
}

impl<I: Iterator> IntoIterator for LazyVec<I> {
    type IntoIter = IntoIter<I>;
    type Item = I::Item;

    fn into_iter(self) -> IntoIter<I> {
        match unsafe { self.0.into_inner() } {
            LazyVecInner::Iter(iter) => IntoIter::Iter(iter),
            LazyVecInner::Vec(v) => IntoIter::VecIter(v.into_iter()),
        }
    }
}

impl<I: Iterator> ::std::fmt::Debug for LazyVec<I>
    where I::Item: ::std::fmt::Debug
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
