use std::cell::RefCell;

pub use std::cell::{Ref, RefMut};

#[derive(Debug)]
pub struct Shared<T> {
    inner: RefCell<T>,
}

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Shared {
            inner: RefCell::new(value),
        }
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
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
