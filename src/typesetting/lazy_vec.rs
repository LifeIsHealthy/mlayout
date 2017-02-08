use std::mem;
use std::cell::UnsafeCell;

pub enum LazyVecInner<I, T> {
    Iter(I),
    Vec(Vec<T>),
}

impl<T, I> LazyVecInner<I, T>
    where I: Iterator<Item = T>
{
    // If the content is a `Vec` this must not change `self` in any way!
    fn replace_with_vec(&mut self) {
        let vec = if let LazyVecInner::Iter(ref mut iter) = *self {
            iter.collect()
        } else {
            return;
        };
        mem::replace(self, LazyVecInner::Vec(vec));
    }
}

pub struct LazyVec<I, T>(UnsafeCell<LazyVecInner<I, T>>);

impl<I, T> LazyVec<I, T>
    where I: Iterator<Item = T>
{
    pub fn with_iter(iter: I) -> Self {
        LazyVec(UnsafeCell::new(LazyVecInner::Iter(iter)))
    }

    pub fn with_vec(vec: Vec<T>) -> Self {
        LazyVec(UnsafeCell::new(LazyVecInner::Vec(vec)))
    }

    pub fn as_slice(&self) -> &[T] {
        let mut inner = unsafe { &mut *self.0.get() };
        inner.replace_with_vec();
        match *inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(ref vec) => &vec[..],
        }
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T> {
        let mut inner = unsafe { &mut *self.0.get() };
        inner.replace_with_vec();
        match *inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(ref mut vec) => vec,
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        let mut inner = unsafe { self.0.into_inner() };
        inner.replace_with_vec();
        match inner {
            LazyVecInner::Iter(_) => panic!("LazyVec is in inconsistent state."),
            LazyVecInner::Vec(vec) => vec,
        }
    }
}

pub enum IntoIter<I, T> {
    Iter(I),
    VecIter(::std::vec::IntoIter<T>),
}

impl<I, T> Iterator for IntoIter<I, T>
    where I: Iterator<Item = T>
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match *self {
            IntoIter::Iter(ref mut iter) => iter.next(),
            IntoIter::VecIter(ref mut iter) => iter.next(),
        }
    }
}

impl<I, T> IntoIterator for LazyVec<I, T>
    where I: Iterator<Item = T>
{
    type IntoIter = IntoIter<I, T>;
    type Item = T;

    fn into_iter(self) -> IntoIter<I, T> {
        match unsafe { self.0.into_inner() } {
            LazyVecInner::Iter(iter) => IntoIter::Iter(iter),
            LazyVecInner::Vec(v) => IntoIter::VecIter(v.into_iter()),
        }
    }
}

impl<I, T> ::std::fmt::Debug for LazyVec<I, T>
    where T: ::std::fmt::Debug,
          I: Iterator<Item = T>
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
