use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub use std::cell::{Ref, RefMut};
pub use std::{assert as assume, unreachable};

#[derive(Debug)]
pub(super) struct RcCell<T> {
    inner: Rc<RefCell<T>>,
}

#[derive(Debug)]
pub(super) struct WeakCell<T> {
    inner: Weak<RefCell<T>>,
}

impl<T> RcCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
        }
    }

    // pub fn ptr_eq(&self, rhs: &Self) -> bool {
    //     Rc::ptr_eq(&self.inner, &rhs.inner)
    // }

    pub fn get(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    // pub fn deep_clone(&self) -> Self
    // where
    //     T: Clone,
    // {
    //     Self {
    //         inner: Rc::new(RefCell::new(T::clone(&*self.inner.borrow()))),
    //     }
    // }

    // Methods below may allow to modify reference counts
    // so they must take `&mut self` though the implementation doesn't requires it.

    pub fn get_mut(&mut self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    pub fn shallow_clone(&mut self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }

    pub fn downgrade(&mut self) -> WeakCell<T> {
        WeakCell {
            inner: Rc::downgrade(&self.inner),
        }
    }
}

impl<T> WeakCell<T> {
    // pub fn upgrade(&mut self) -> RcCell<T> {
    //     RcCell {
    //         inner: self.inner.upgrade().unwrap(),
    //     }
    // }
}

#[derive(Debug)]
pub struct RefKey<'a, T>(RefMut<'a, T>);

impl<'a, T> std::ops::Deref for RefKey<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, T: std::fmt::Display> std::fmt::Display for RefKey<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub fn map_ref<T, U, F: FnOnce(&T) -> &U>(orig: Ref<'_, T>, f: F) -> Ref<'_, U> {
    Ref::map(orig, f)
}

pub fn split_ref<T, U, V, F: FnOnce(&T) -> (&U, &V)>(
    orig: Ref<'_, T>,
    f: F,
) -> (Ref<'_, U>, Ref<'_, V>) {
    Ref::map_split(orig, f)
}

pub fn map_mut<T, U, F: FnOnce(&mut T) -> &mut U>(orig: RefMut<'_, T>, f: F) -> RefMut<'_, U> {
    RefMut::map(orig, f)
}

pub fn split_mut<T, U, V, F: FnOnce(&mut T) -> (&mut U, &mut V)>(
    orig: RefMut<'_, T>,
    f: F,
) -> (RefMut<'_, U>, RefMut<'_, V>) {
    RefMut::map_split(orig, f)
}

pub fn mut_to_key<'a, T>(orig: RefMut<'a, T>) -> RefKey<'a, T> {
    RefKey(orig)
}
