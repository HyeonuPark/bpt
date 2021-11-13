use std::borrow::Borrow;
use std::cmp::Ord;

use crate::checked::{Ref, RefKey, RefMut};
use crate::leaf::Leaf;

pub(crate) trait Node<K, V, const CAP: usize>: Sized {
    fn head(&self) -> &Leaf<K, V, CAP>;

    fn tail(&self) -> &Leaf<K, V, CAP>;

    fn head_mut(&mut self) -> &mut Leaf<K, V, CAP>;

    fn tail_mut(&mut self) -> &mut Leaf<K, V, CAP>;

    fn get<Q: Ord>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: Borrow<Q>;

    fn get_mut<Q: Ord>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: Borrow<Q>;

    fn insert(&mut self, new_entry: (K, V)) -> (Option<(K, V)>, Option<Self>);

    fn remove<Q: Ord>(&mut self, query: &Q) -> Option<((K, V), bool)>
    where
        K: Borrow<Q>;

    fn balance_or_drain(&mut self, next: &mut Self, lacking_next: bool) -> bool;
}
