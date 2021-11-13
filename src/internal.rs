use std::borrow::Borrow;
use std::cmp::{Ord, Ordering};

use arrayvec::ArrayVec;

use crate::checked::{self, Ref, RefKey, RefMut};
use crate::insert_or_split;
use crate::leaf::Leaf;
use crate::node::Node;

#[derive(Debug)]
pub(crate) struct Internal<K, V, const CAP: usize> {
    head: Leaf<K, V, CAP>,
    tail: Leaf<K, V, CAP>,
    children: Children<K, V, CAP>,
}

#[derive(Debug)]
enum Children<K, V, const CAP: usize> {
    Internal(ArrayVec<Box<Internal<K, V, CAP>>, CAP>),
    Leaf(ArrayVec<Leaf<K, V, CAP>, CAP>),
}

impl<K: Ord, V, const CAP: usize> Internal<K, V, CAP> {
    pub fn new(mut leaf: Leaf<K, V, CAP>) -> Self {
        Self {
            head: leaf.shallow_clone(),
            tail: leaf.shallow_clone(),
            children: Children::Leaf([leaf].into_iter().collect()),
        }
    }

    pub fn wrap(mut left: Box<Self>, mut right: Box<Self>) -> Self {
        Self {
            head: left.head.shallow_clone(),
            tail: right.tail.shallow_clone(),
            children: Children::Internal([left, right].into_iter().collect()),
        }
    }

    pub fn check_lower<Q: Ord>(&self, query: &Q) -> Option<()>
    where
        K: Borrow<Q>,
    {
        if (*self.head.first()).borrow() <= query {
            Some(())
        } else {
            None
        }
    }

    pub fn pop_depth(&mut self) -> Option<Box<Self>> {
        match &mut self.children {
            Children::Internal(children) if children.len() == 1 => children.pop(),
            _ => None,
        }
    }

    fn child_idx<Q: Ord>(&self, query: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
    {
        debug_assert!(
            (*self.head.first()).borrow() <= query,
            "This should be checked on the upper level"
        );

        if (*self.tail.last()).borrow() < query {
            return None;
        }

        Some(match &self.children {
            Children::Internal(children) => find_idx(children, query),
            Children::Leaf(children) => find_idx(children, query),
        })
    }
}

impl<K: Ord, V, const CAP: usize> Node<K, V, CAP> for Box<Internal<K, V, CAP>> {
    fn head(&self) -> &Leaf<K, V, CAP> {
        &self.head
    }

    fn tail(&self) -> &Leaf<K, V, CAP> {
        &self.tail
    }

    fn head_mut(&mut self) -> &mut Leaf<K, V, CAP> {
        &mut self.head
    }

    fn tail_mut(&mut self) -> &mut Leaf<K, V, CAP> {
        &mut self.tail
    }

    fn get<Q: Ord>(&self, query: &Q) -> Option<(Ref<'_, K>, Ref<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
    {
        let idx = self.child_idx(query)?;
        match &self.children {
            Children::Internal(children) => children[idx].get(query),
            Children::Leaf(children) => children[idx].get(query),
        }
    }

    fn get_mut<Q: Ord>(&mut self, query: &Q) -> Option<(RefKey<'_, K>, RefMut<'_, V>)>
    where
        K: std::borrow::Borrow<Q>,
    {
        let idx = self.child_idx(query)?;
        match &mut self.children {
            Children::Internal(children) => children[idx].get_mut(query),
            Children::Leaf(children) => children[idx].get_mut(query),
        }
    }

    fn insert(&mut self, new_entry: (K, V)) -> (Option<(K, V)>, Option<Self>) {
        fn insert_entry<N: Node<K, V, CAP>, K: Ord, V, const CAP: usize>(
            nodes: &mut ArrayVec<N, CAP>,
            entry: (K, V),
            prev_out: &mut Option<(K, V)>,
        ) -> Option<(ArrayVec<N, CAP>, Leaf<K, V, CAP>, Leaf<K, V, CAP>)> {
            let idx = find_idx(&nodes, &entry.0);
            let child = &mut nodes[idx];

            let (prev, new_node) = child.insert(entry);
            *prev_out = prev;

            let mut children = insert_or_split(nodes, idx + 1, new_node?)?;
            let head = children.first_mut().map_or_else(
                || checked::unreachable!("children shouldn't be empty"),
                |n| n.head_mut().shallow_clone(),
            );
            let tail = children.last_mut().map_or_else(
                || checked::unreachable!("children shouldn't be empty"),
                |n| n.tail_mut().shallow_clone(),
            );

            Some((children, head, tail))
        }

        let mut prev = None;

        let new_node = match &mut self.children {
            Children::Internal(children) => {
                let res = insert_entry(children, new_entry, &mut prev);
                if prev.is_none() {
                    self.tail = children.last_mut().unwrap().tail_mut().shallow_clone();
                }
                if let Some((children, head, tail)) = res {
                    Internal {
                        children: Children::Internal(children),
                        head,
                        tail,
                    }
                } else {
                    return (prev, None);
                }
            }
            Children::Leaf(children) => {
                let res = insert_entry(children, new_entry, &mut prev);
                if prev.is_none() {
                    self.tail = children.last_mut().unwrap().tail_mut().shallow_clone();
                }
                if let Some((children, head, tail)) = res {
                    Internal {
                        children: Children::Leaf(children),
                        head,
                        tail,
                    }
                } else {
                    return (prev, None);
                }
            }
        };

        (prev, Some(Box::new(new_node)))
    }

    fn remove<Q: Ord>(&mut self, query: &Q) -> Option<((K, V), bool)>
    where
        K: Borrow<Q>,
    {
        fn remove_entry<N: Node<K, V, CAP>, Q: Ord, K: Ord + Borrow<Q>, V, const CAP: usize>(
            children: &mut ArrayVec<N, CAP>,
            idx: usize,
            query: &Q,
            tail: &mut Leaf<K, V, CAP>,
        ) -> Option<((K, V), bool)> {
            let (entry, need_merge) = children[idx].remove(query)?;

            if !need_merge {
                *tail = children.last_mut().unwrap().tail_mut().shallow_clone();
                return Some((entry, false));
            }

            let (lacking_next, left_idx) = match idx {
                0 => (false, 0),
                _ => (true, idx - 1),
            };
            let (left, right) = match &mut children[left_idx..] {
                [left, right, ..] => (left, right),
                // only root node can have single child
                _ => return Some((entry, true)),
            };
            let drained = left.balance_or_drain(right, lacking_next);

            if drained {
                children.remove(left_idx + 1);
            }

            let b = CAP / 2 + 1;
            *tail = children.last_mut().unwrap().tail_mut().shallow_clone();
            Some((entry, children.len() < b))
        }

        let idx = self.child_idx(query)?;

        match &mut self.children {
            Children::Internal(children) => remove_entry(children, idx, query, &mut self.tail),
            Children::Leaf(children) => remove_entry(children, idx, query, &mut self.tail),
        }
    }

    fn balance_or_drain(&mut self, next_node: &mut Self, lacking_next: bool) -> bool {
        fn do_balance_or_drain<N: Node<K, V, CAP>, K, V, const CAP: usize>(
            this: &mut ArrayVec<N, CAP>,
            next: &mut ArrayVec<N, CAP>,
            lacking_next: bool,
        ) -> bool {
            let b = CAP / 2 + 1;

            if lacking_next && this.len() > b {
                next.insert(0, this.pop().unwrap());
                return false;
            }

            if !lacking_next && next.len() > b {
                this.push(next.remove(0));
                return false;
            }

            this.extend(next.drain(..));
            true
        }

        fn with_tail_head<N: Node<K, V, CAP>, K, V, const CAP: usize>(
            this: &mut ArrayVec<N, CAP>,
            next: &mut ArrayVec<N, CAP>,
            lacking_next: bool,
        ) -> (Leaf<K, V, CAP>, Option<Leaf<K, V, CAP>>) {
            let next_head = match do_balance_or_drain(this, next, lacking_next) {
                true => None,
                false => Some(next.first_mut().unwrap().head_mut().shallow_clone()),
            };
            let this_tail = this.last_mut().unwrap().tail_mut().shallow_clone();

            (this_tail, next_head)
        }

        let (this_tail, next_head) = match (&mut self.children, &mut next_node.children) {
            (Children::Internal(this), Children::Internal(next)) => {
                with_tail_head(this, next, lacking_next)
            }
            (Children::Leaf(this), Children::Leaf(next)) => {
                with_tail_head(this, next, lacking_next)
            }
            _ => checked::unreachable!("All the leafs must be in the same depth"),
        };

        self.tail = this_tail;

        match next_head {
            Some(head) => {
                next_node.head = head;
                false
            }
            None => true,
        }
    }
}

fn find_idx<Q: Ord, K: Ord + Borrow<Q>, V, const CAP: usize>(
    slice: &[impl Node<K, V, CAP>],
    query: &Q,
) -> usize {
    checked::assume!(slice.len() > 0);

    for (idx, node) in slice[1..].iter().enumerate() {
        return match (*node.head().first()).borrow().cmp(query) {
            Ordering::Greater => idx,
            Ordering::Equal => idx + 1,
            Ordering::Less => continue,
        };
    }

    slice.len() - 1
}
