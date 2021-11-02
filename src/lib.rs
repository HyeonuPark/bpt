use std::cmp::{Ord, Ordering};
use std::fmt;
use std::mem;
use std::rc::{Rc, Weak};

use arrayvec::ArrayVec;

#[cfg_attr(feature = "unchecked", path = "shared_unchecked.rs")]
#[cfg_attr(not(feature = "unchecked"), path = "shared_checked.rs")]
mod shared;

use shared::{Ref, RefKey, RefMut, Shared};

const B: usize = 12;
const CAPACITY: usize = 2 * B - 1;

#[derive(Debug)]
pub struct BTreeMap<K, V> {
    root: Root<K, V>,
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

impl<K: Ord, V> BTreeMap<K, V> {
    #[inline]
    pub fn new() -> Self {
        BTreeMap {
            root: Root::Empty,
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
        match &self.root {
            Root::Empty => None,
            Root::Internal(node) => node.get(query),
            Root::Leaf(node) => node.get(query),
        }
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
        match &mut self.root {
            Root::Empty => None,
            Root::Internal(node) => node.get_mut(query),
            Root::Leaf(node) => node.get_mut(query),
        }
    }

    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.insert_entry(key, value).map(|(_, v)| v)
    }

    #[inline]
    pub fn insert_entry(&mut self, key: K, value: V) -> Option<(K, V)> {
        let root = mem::replace(&mut self.root, Root::Empty);
        let (root, res) = root.insert((key, value));
        self.root = root;

        if res.is_none() {
            self.length += 1;
        }

        res
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
        let root = mem::replace(&mut self.root, Root::Empty);
        let (root, res) = root.remove(query);
        self.root = root;

        if res.is_some() {
            self.length -= 1;

            if self.length == 0 {
                self.root = Root::Empty;
            }
        }

        res
    }
}

#[derive(Debug)]
enum Root<K, V> {
    Internal(Internal<K, V>),
    Leaf(Leaf<K, V>),
    Empty,
}

type Internal<K, V> = Box<InternalNode<K, V>>;

struct Leaf<K, V>(Rc<Shared<LeafNode<K, V>>>);

#[derive(Debug)]
struct LeafWeak<K, V>(Weak<Shared<LeafNode<K, V>>>);

#[derive(Debug)]
struct InternalNode<K, V> {
    children: ArrayVec<Child<K, V>, CAPACITY>,
}

#[derive(Debug)]
struct Child<K, V> {
    head_leaf: Leaf<K, V>,
    internal: Option<Internal<K, V>>,
}

#[derive(Debug)]
struct LeafNode<K, V> {
    entries: ArrayVec<(K, V), CAPACITY>,
    prev: Option<LeafWeak<K, V>>,
    next: Option<LeafWeak<K, V>>,
}

impl<K: Ord, V> Root<K, V> {
    fn insert(self, entry: (K, V)) -> (Self, Option<(K, V)>) {
        match self {
            Root::Empty => (
                Root::Leaf(Leaf::new(LeafNode {
                    entries: [entry].into_iter().collect(),
                    prev: None,
                    next: None,
                })),
                None,
            ),
            Root::Leaf(mut node) => {
                let (prev, new_node) = node.insert(entry);
                if let Some(new_node) = new_node {
                    let children = [node, new_node]
                        .into_iter()
                        .map(|node| Child {
                            internal: None,
                            head_leaf: node,
                        })
                        .collect();
                    (Root::Internal(Box::new(InternalNode { children })), prev)
                } else {
                    (Root::Leaf(node), prev)
                }
            }
            Root::Internal(mut node) => {
                let (prev, new_node) = node.insert(entry);
                if let Some(new_node) = new_node {
                    let children = [node, new_node]
                        .into_iter()
                        .map(|node| Child {
                            head_leaf: node.head_leaf(),
                            internal: Some(node),
                        })
                        .collect();
                    (Root::Internal(Box::new(InternalNode { children })), prev)
                } else {
                    (Root::Internal(node), prev)
                }
            }
        }
    }

    fn remove<Q>(self, query: &Q) -> (Self, Option<(K, V)>)
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        match self {
            Root::Empty => (Root::Empty, None),
            Root::Leaf(mut node) => match node.remove(query) {
                None => (Root::Leaf(node), None),
                Some((key, value, _)) => (Root::Leaf(node), Some((key, value))),
            },
            Root::Internal(mut node) => match node.remove(query) {
                None => (Root::Internal(node), None),
                Some((key, value, _)) => {
                    let res = Some((key, value));

                    if node.children.len() > 1 {
                        (Root::Internal(node), res)
                    } else {
                        let only_child = node.children.pop().unwrap();
                        if let Some(node) = only_child.internal {
                            (Root::Internal(node), res)
                        } else {
                            (Root::Leaf(only_child.head_leaf), res)
                        }
                    }
                }
            },
        }
    }
}

impl<K: Ord, V> InternalNode<K, V> {
    fn head_leaf(&self) -> Leaf<K, V> {
        Leaf(self.children[0].head_leaf.0.clone())
    }

    fn child_idx<Q>(&self, query: &Q) -> usize
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let children = &self.children;
        let idx = linear_search_by(&children[1..], |child| {
            borrow(&*child.head_leaf.head()).cmp(query)
        });
        match idx {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }

    fn get<Q>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let idx = self.child_idx(query);
        let child = &self.children[idx];

        match &child.internal {
            Some(node) => node.get(query),
            None => child.head_leaf.get(query),
        }
    }

    fn get_mut<Q>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let idx = self.child_idx(query);
        let child = &mut self.children[idx];

        match &mut child.internal {
            Some(node) => node.get_mut(query),
            None => child.head_leaf.get_mut(query),
        }
    }

    fn insert(&mut self, entry: (K, V)) -> (Option<(K, V)>, Option<Box<Self>>) {
        let children = &mut self.children;
        let idx =
            match linear_search_by(&children[1..], |child| child.head_leaf.head().cmp(&entry.0)) {
                Ok(idx) => {
                    // since it's from children[1..]
                    let prev = children[idx + 1].head_leaf.replace_head(entry);
                    return (Some(prev), None);
                }
                Err(idx) => idx,
            };

        let child = &mut children[idx];

        if let Some(internal) = &mut child.internal {
            let (prev, new_node) = internal.insert(entry);
            let child = new_node.map(|node| Child {
                head_leaf: node.head_leaf(),
                internal: Some(node),
            });
            (prev, self.handle_new_child(idx, child))
        } else {
            let (prev, new_leaf) = child.head_leaf.insert(entry);
            let child = new_leaf.map(|leaf| Child {
                head_leaf: leaf,
                internal: None,
            });
            (prev, self.handle_new_child(idx, child))
        }
    }

    fn handle_new_child(&mut self, idx: usize, child: Option<Child<K, V>>) -> Option<Box<Self>> {
        Some(Box::new(InternalNode {
            children: insert_or_split(&mut self.children, idx + 1, child?)?,
        }))
    }

    // returns (key, value, need_merge)
    fn remove<Q>(&mut self, query: &Q) -> Option<(K, V, bool)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let idx = self.child_idx(query);
        let children = &mut self.children;
        let child = &mut children[idx];

        let (key, value, need_merge) = if let Some(internal) = &mut child.internal {
            internal.remove(query)?
        } else {
            child.head_leaf.remove(query)?
        };

        if !need_merge {
            return Some((key, value, false));
        }

        let (lacking_next, start) = if idx != 0 {
            (true, idx - 1)
        } else {
            (false, 0)
        };
        let (left, right) = match &mut children[start..] {
            [left, right, ..] => (left, right),
            _ => {
                unreachable!("Internal node should contains more than 1 children before .remove()")
            }
        };
        let consumed = match (&mut left.internal, &mut right.internal) {
            (Some(left_int), Some(right_int)) => {
                let consumed = left_int.balance_or_drain(right_int, lacking_next);
                if !consumed {
                    right.head_leaf = right_int.head_leaf();
                }
                consumed
            }
            (None, None) => left
                .head_leaf
                .balance_or_drain(&mut right.head_leaf, lacking_next),
            _ => unreachable!("Leaf nodes should be all in same level"),
        };

        if consumed {
            children.remove(start + 1);
        }

        Some((key, value, children.len() < B))
    }

    // returns true if next is drained
    fn balance_or_drain(&mut self, next: &mut Self, lacking_next: bool) -> bool {
        if lacking_next && self.children.len() > B {
            next.children.insert(0, self.children.pop().unwrap());
            return false;
        }

        if !lacking_next && next.children.len() > B {
            self.children.push(next.children.remove(0));
            return false;
        }

        self.children.extend(next.children.drain(..));
        true
    }
}

#[cfg_attr(feature = "unchecked", allow(unused_mut))]
impl<K: Ord, V> Leaf<K, V> {
    fn new(node: LeafNode<K, V>) -> Self {
        Self(Rc::new(Shared::new(node)))
    }

    fn weak(&self) -> LeafWeak<K, V> {
        LeafWeak(Rc::downgrade(&self.0))
    }

    fn head(&self) -> Ref<'_, K> {
        shared::map_ref(self.0.borrow(), |node| &node.entries.first().unwrap().0)
    }

    fn replace_head(&mut self, entry: (K, V)) -> (K, V) {
        let mut node = self.0.borrow_mut();
        let head = node.entries.first_mut().unwrap();
        debug_assert!(entry.0 == head.0);
        mem::replace(head, entry)
    }

    fn get<Q>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let entries = shared::map_ref(self.0.borrow(), |n| &n.entries);
        let idx = linear_search_by(&entries, |entry| borrow(&entry.0).cmp(query)).ok()?;
        let entry = shared::map_ref(entries, |entries| &entries[idx]);
        Some(shared::split_ref(entry, |entry| (&entry.0, &entry.1)))
    }

    fn get_mut<Q>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let entries = shared::map_mut(self.0.borrow_mut(), |n| &mut n.entries);
        let idx = linear_search_by(&entries, |entry| borrow(&entry.0).cmp(query)).ok()?;
        let entry = shared::map_mut(entries, |entries| &mut entries[idx]);
        let (key, value) = shared::split_mut(entry, |entry| (&mut entry.0, &mut entry.1));
        Some((shared::mut_to_key(key), value))
    }

    fn insert(&mut self, entry: (K, V)) -> (Option<(K, V)>, Option<Self>) {
        let mut node = self.0.borrow_mut();
        let entries = &mut node.entries;

        let idx = match linear_search_by(&entries, |elem| elem.0.cmp(&entry.0)) {
            Ok(idx) => return (Some(mem::replace(&mut entries[idx], entry)), None),
            Err(idx) => idx,
        };

        let new_entries = match insert_or_split(entries, idx, entry) {
            None => return (None, None),
            Some(entries) => entries,
        };

        let next_next = node.next.take();
        drop(node);
        let this_weak = self.weak();

        let next_leaf = Leaf::new(LeafNode {
            entries: new_entries,
            prev: Some(this_weak),
            next: next_next,
        });
        self.0.borrow_mut().next = Some(next_leaf.weak());

        (None, Some(next_leaf))
    }

    // returns (key, value, need_merge)
    fn remove<Q>(&mut self, query: &Q) -> Option<(K, V, bool)>
    where
        K: std::borrow::Borrow<Q>,
        Q: Ord,
    {
        let mut node = self.0.borrow_mut();
        let entries = &mut node.entries;
        let idx = linear_search_by(&entries, |entry| borrow(&entry.0).cmp(query)).ok()?;
        let (key, value) = entries.remove(idx);
        Some((key, value, entries.len() < B))
    }

    // returns true if next is drained
    fn balance_or_drain(&mut self, next: &mut Self, lacking_next: bool) -> bool {
        let mut this = self.0.borrow_mut();
        let mut next = next.0.borrow_mut();

        if lacking_next && this.entries.len() > B {
            next.entries.insert(0, this.entries.pop().unwrap());
            return false;
        }

        if !lacking_next && next.entries.len() > B {
            this.entries.push(next.entries.remove(0));
            return false;
        }

        this.entries.extend(next.entries.drain(..));
        this.next = next.next.take();
        true
    }
}

fn borrow<T: std::borrow::Borrow<Q>, Q>(owned: &T) -> &Q {
    std::borrow::Borrow::borrow(owned)
}

fn insert_or_split<T>(
    buf: &mut ArrayVec<T, CAPACITY>,
    idx: usize,
    new: T,
) -> Option<ArrayVec<T, CAPACITY>> {
    if !buf.is_full() {
        buf.insert(idx, new);
        return None;
    }

    let mut new_buf = ArrayVec::new();
    let (insert_to, insert_idx);
    if idx < B {
        new_buf.extend(buf.drain(B - 1..));
        insert_to = buf;
        insert_idx = idx;
    } else {
        new_buf.extend(buf.drain(B..));
        insert_to = &mut new_buf;
        insert_idx = idx - B;
    }
    insert_to.insert(insert_idx, new);

    Some(new_buf)
}

fn linear_search_by<T, F: FnMut(&T) -> Ordering>(slice: &[T], mut f: F) -> Result<usize, usize> {
    for (idx, elem) in slice.iter().enumerate() {
        return match f(elem) {
            Ordering::Greater => Err(idx),
            Ordering::Equal => Ok(idx),
            Ordering::Less => continue,
        };
    }

    Err(slice.len())
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Leaf<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (k, _) in &self.0.borrow().entries {
            write!(f, " {:?}", k)?;
        }
        f.write_str(" ]")?;

        Ok(())
    }
}

#[test]
fn std_compare() {
    let mut m1 = std::collections::BTreeMap::new();
    let mut m2 = BTreeMap::new();

    let nums: Vec<u16> = std::iter::repeat_with(rand::random)
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
