use std::borrow::Borrow;
use std::cmp::{Ord, Ordering};
use std::fmt;
use std::mem;

use arrayvec::ArrayVec;

use crate::checked::{self, RcCell, Ref, RefKey, RefMut, WeakCell};
use crate::insert_or_split;
use crate::node::Node;

pub(crate) struct Leaf<K, V, const CAP: usize>(RcCell<LeafData<K, V, CAP>>);

#[derive(Debug)]
struct LeafData<K, V, const CAP: usize> {
    entries: ArrayVec<(K, V), CAP>,
    prev: Option<WeakCell<Self>>,
    next: Option<WeakCell<Self>>,
}

impl<K: Clone, V: Clone, const CAP: usize> Clone for LeafData<K, V, CAP> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            prev: None,
            next: None,
        }
    }
}

impl<K, V, const CAP: usize> Leaf<K, V, CAP> {
    pub fn new(entry: (K, V)) -> Self {
        Leaf(RcCell::new(LeafData {
            entries: [entry].into_iter().collect(),
            prev: None,
            next: None,
        }))
    }

    // pub fn ptr_eq(&self, rhs: &Self) -> bool {
    //     RcCell::ptr_eq(&self.0, &rhs.0)
    // }

    pub fn first(&self) -> Ref<'_, K> {
        checked::map_ref(self.0.get(), |this| match this.entries.first() {
            Some(entry) => &entry.0,
            None => checked::unreachable!("Leaf node can't be empty"),
        })
    }

    pub fn last(&self) -> Ref<'_, K> {
        checked::map_ref(self.0.get(), |this| match this.entries.last() {
            Some(entry) => &entry.0,
            None => checked::unreachable!("Leaf node can't be empty"),
        })
    }

    pub fn shallow_clone(&mut self) -> Self {
        Self(RcCell::shallow_clone(&mut self.0))
    }
}

// impl<K: Clone, V: Clone, const CAP: usize> Leaf<K, V, CAP> {
//     /// Deep clone doesn't preserve links between nodes.
//     pub fn deep_clone(&self) -> Self {
//         Self(RcCell::deep_clone(&self.0))
//     }
// }

impl<K: Ord, V, const CAP: usize> Node<K, V, CAP> for Leaf<K, V, CAP> {
    fn head(&self) -> &Leaf<K, V, CAP> {
        self
    }

    fn tail(&self) -> &Leaf<K, V, CAP> {
        self
    }

    fn head_mut(&mut self) -> &mut Leaf<K, V, CAP> {
        self
    }

    fn tail_mut(&mut self) -> &mut Leaf<K, V, CAP> {
        self
    }

    fn get<Q: Ord>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: Borrow<Q>,
    {
        let entries = checked::map_ref(self.0.get(), |this| &this.entries);
        let idx = query_idx(&entries, query).ok()?;
        let entry = checked::map_ref(entries, |entries| &entries[idx]);
        Some(checked::split_ref(entry, |entry| (&entry.0, &entry.1)))
    }

    fn get_mut<Q: Ord>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: Borrow<Q>,
    {
        let entries = checked::map_mut(self.0.get_mut(), |this| &mut this.entries);
        let idx = query_idx(&entries, query).ok()?;
        let entry = checked::map_mut(entries, |entries| &mut entries[idx]);
        let (key, value) = checked::split_mut(entry, |entry| (&mut entry.0, &mut entry.1));
        Some((checked::mut_to_key(key), value))
    }

    fn insert(&mut self, new_entry: (K, V)) -> (Option<(K, V)>, Option<Self>) {
        let mut this = self.0.get_mut();
        let entries = &mut this.entries;

        let idx = match query_idx(&entries, &new_entry.0) {
            Ok(idx) => {
                let entry = mem::replace(&mut entries[idx], new_entry);
                return (Some(entry), None);
            }
            Err(idx) => idx,
        };

        let new_entries = match insert_or_split(entries, idx, new_entry) {
            Some(entries) => entries,
            None => return (None, None),
        };

        let next_next = this.next.take();
        drop(this);
        let this_weak = self.0.downgrade();

        let mut next = Leaf(RcCell::new(LeafData {
            entries: new_entries,
            prev: Some(this_weak),
            next: next_next,
        }));
        self.0.get_mut().next = Some(next.0.downgrade());

        (None, Some(next))
    }

    fn remove<Q: Ord>(&mut self, query: &Q) -> Option<((K, V), bool)>
    where
        K: Borrow<Q>,
    {
        let mut this = self.0.get_mut();
        let entries = &mut this.entries;
        let idx = query_idx(&entries, query).ok()?;
        let b = CAP / 2 + 1;
        Some((entries.remove(idx), entries.len() < b))
    }

    fn balance_or_drain(&mut self, next: &mut Self, lacking_next: bool) -> bool {
        let mut this = self.0.get_mut();
        let mut next = next.0.get_mut();

        let b = CAP / 2 + 1;

        if lacking_next && this.entries.len() > b {
            next.entries.insert(
                0,
                this.entries
                    .pop()
                    .unwrap_or_else(|| checked::unreachable!("leafs can't be empty")),
            );
            return false;
        }

        if !lacking_next && next.entries.len() > b {
            this.entries.push(next.entries.remove(0));
            return false;
        }

        this.entries.extend(next.entries.drain(..));
        this.next = next.next.take();
        true
    }
}

fn query_idx<K: Borrow<Q>, V, Q: Ord>(slice: &[(K, V)], query: &Q) -> Result<usize, usize> {
    for (idx, (key, _)) in slice.iter().enumerate() {
        return match key.borrow().cmp(query) {
            Ordering::Greater => Err(idx),
            Ordering::Equal => Ok(idx),
            Ordering::Less => continue,
        };
    }

    Err(slice.len())
}

impl<K: fmt::Debug, V: fmt::Debug, const CAP: usize> fmt::Debug for Leaf<K, V, CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "[")?;
        for (k, _) in &self.0.get().entries {
            write!(f, " {:?}", k)?;
        }
        write!(f, "{}", " ]")?;
        Ok(())
    }
}
