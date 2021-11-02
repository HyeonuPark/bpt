//! Unchecked version of the shared mutable container.
//!
//! If the logic is correct with the checked version,
//! it should also be sound with this unchecked version
//! as all the checks in there are redundant and always panic when failed.

use std::cell::UnsafeCell;

pub type Ref<'a, T> = &'a T;
pub type RefMut<'a, T> = &'a mut T;
pub type RefKey<'a, T> = &'a T;

#[derive(Debug)]
pub struct Shared<T> {
    inner: UnsafeCell<T>,
}

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Shared {
            inner: UnsafeCell::new(value),
        }
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        unsafe { &*self.inner.get() }
    }

    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        unsafe { &mut *self.inner.get() }
    }
}

pub fn map_ref<T, U, F: FnOnce(&T) -> &U>(orig: Ref<'_, T>, f: F) -> Ref<'_, U> {
    f(orig)
}

pub fn split_ref<T, U, V, F: FnOnce(&T) -> (&U, &V)>(
    orig: Ref<'_, T>,
    f: F,
) -> (Ref<'_, U>, Ref<'_, V>) {
    f(orig)
}

pub fn map_mut<T, U, F: FnOnce(&mut T) -> &mut U>(orig: RefMut<'_, T>, f: F) -> RefMut<'_, U> {
    f(orig)
}

pub fn split_mut<T, U, V, F: FnOnce(&mut T) -> (&mut U, &mut V)>(
    orig: RefMut<'_, T>,
    f: F,
) -> (RefMut<'_, U>, RefMut<'_, V>) {
    f(orig)
}

pub fn mut_to_key<'a, T>(orig: RefMut<'a, T>) -> RefKey<'a, T> {
    &*orig
}
