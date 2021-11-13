use std::cmp::Ord;

use arrayvec::ArrayVec;

#[cfg_attr(feature = "unchecked", path = "unchecked.rs")]
mod checked;
mod internal;
mod leaf;
mod node;

use checked::{Ref, RefKey, RefMut};
use internal::Internal;
use leaf::Leaf;
use node::Node;

#[derive(Debug)]
pub struct BTreeMap<K, V, const CAP: usize> {
    root: Option<Box<Internal<K, V, CAP>>>,
    length: usize,
}

// The only reason this impls are not automatic is that
// the type contains `RefCell<T>` and `Rc<T>`.
// `RefCell`s are removed with the `"unchecked"` feature.
// All the `Rc<T>`s are local to each BTreeMap and not exposed,
// and all the code which touches the reference count
// requires to hold the `&mut BTreeMap<K, V>` reference.
#[cfg(feature = "std-compat")]
unsafe impl<K: Send, V: Send> Send for BTreeMap<K, V> {}
#[cfg(feature = "std-compat")]
unsafe impl<K: Sync, V: Sync> Sync for BTreeMap<K, V> {}

impl<K: Ord, V, const CAP: usize> BTreeMap<K, V, CAP> {
    #[inline]
    pub fn new() -> Self {
        assert!(CAP % 2 == 1, "Node capacity must be an odd number");
        assert!(CAP > 3, "Node capacity must be larger then 3");

        BTreeMap {
            root: None,
            length: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    pub fn get<Q>(&self, query: &Q) -> Option<Ref<'_, V>>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        self.entry(query).map(|(_, v)| v)
    }

    #[inline]
    pub fn entry<Q>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let root = self.root.as_ref()?;
        root.check_lower(query)?;
        root.get(query)
    }

    #[inline]
    pub fn get_mut<Q>(&mut self, query: &Q) -> Option<RefMut<'_, V>>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        self.entry_mut(query).map(|(_, v)| v)
    }

    #[inline]
    pub fn entry_mut<Q>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let root = self.root.as_mut()?;
        root.check_lower(query)?;
        root.get_mut(query)
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_entry(key, value).map(|(_, v)| v)
    }

    #[inline]
    pub fn insert_entry(&mut self, key: K, value: V) -> Option<(K, V)> {
        let (prev, root) = match self.root.take() {
            Some(mut node) => {
                let (prev, new_node) = node.insert((key, value));
                if let Some(new_node) = new_node {
                    let parent = Internal::wrap(node, new_node);
                    (prev, Box::new(parent))
                } else {
                    (prev, node)
                }
            }
            None => (None, Box::new(Internal::new(Leaf::new((key, value))))),
        };
        self.root = Some(root);

        if prev.is_none() {
            self.length += 1;
        }

        prev
    }

    #[inline]
    pub fn remove<Q>(&mut self, query: &Q) -> Option<V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        self.remove_entry(query).map(|(_, v)| v)
    }

    #[inline]
    pub fn remove_entry<Q>(&mut self, query: &Q) -> Option<(K, V)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let root = self.root.as_mut()?;
        root.check_lower(query)?;
        let (entry, need_merge) = root.remove(query)?;

        self.length -= 1;

        if need_merge {
            if self.length == 0 {
                self.root = None;
            } else if let Some(node) = root.pop_depth() {
                self.root = Some(node)
            }
        }

        Some(entry)
    }
}

fn insert_or_split<T, const CAP: usize>(
    buf: &mut ArrayVec<T, CAP>,
    idx: usize,
    new: T,
) -> Option<ArrayVec<T, CAP>> {
    if !buf.is_full() {
        buf.insert(idx, new);
        return None;
    }

    let mut new_buf = ArrayVec::new();
    let b = CAP / 2 + 1;

    if idx < b {
        new_buf.extend(buf.drain(b - 1..));
        buf.insert(idx, new);
    } else {
        new_buf.extend(buf.drain(b..));
        new_buf.insert(idx - b, new);
    }

    Some(new_buf)
}

#[test]
fn check_same_behavior_with_std_btreemap() {
    let mut m1 = std::collections::BTreeMap::new();
    let mut m2 = BTreeMap::<_, _, 15>::new();

    let nums: Vec<u32> = std::iter::repeat_with(rand::random)
        .take(1024 * 1024)
        .collect();

    for &n in &nums {
        assert_eq!(m1.insert(n, n), m2.insert(n, n));
    }
    for &n in &nums {
        assert_eq!(m1.get(&n), m2.get(&n).as_deref());
        assert_eq!(
            m1.get(&n.wrapping_add(1)),
            m2.get(&n.wrapping_add(1)).as_deref()
        );
    }
    for &n in &nums {
        assert_eq!(m1.remove(&n), m2.remove(&n));
    }
}
