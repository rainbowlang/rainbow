use std::cell::RefCell;
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::FromIterator;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone)]
pub struct Scope<V, K: Hash + Eq = String>(FramePtr<V, K>);

type FramePtr<V, K> = Rc<RefCell<Frame<V, K>>>;
type Locals<V, K> = HashMap<K, Rc<V>>;

#[derive(Debug, Clone, PartialEq)]
struct Frame<V, K: Hash + Eq> {
    locals: Locals<V, K>,
    parent: Option<FramePtr<V, K>>,
}

impl<V, K> Scope<V, K>
where
    K: Hash + Eq + Clone + Debug,
{
    pub fn new() -> Scope<V, K> {
        Scope(Frame::new(HashMap::with_capacity(64), None))
    }

    pub fn new_child(&self) -> Scope<V, K> {
        Scope(Frame::new(HashMap::with_capacity(16), Some(self.0.clone())))
    }

    pub fn new_child_with_capacity(&self, cap: usize) -> Scope<V, K> {
        Scope(Frame::new(
            HashMap::with_capacity(cap),
            Some(self.0.clone()),
        ))
    }

    pub fn insert(&mut self, key: K, val: V) -> Option<Rc<V>> {
        self.0.borrow_mut().locals.insert(key, Rc::new(val))
    }

    pub fn insert_at_root(&mut self, key: K, val: V) -> Option<Rc<V>> {
        self.0.borrow_mut().insert_at_root(key, val)
    }

    pub fn get(&self, key: &K) -> Option<Rc<V>> {
        self.0.borrow().get(key)
    }

    pub fn flatten(&self) -> HashMap<K, Rc<V>> {
        self.0.borrow().flatten()
    }

    pub fn map_clone<F, V2>(&self, f: F) -> Scope<V2, K>
    where
        F: Fn(&V) -> V2,
    {
        Scope(self.0.borrow().map_clone(&f))
    }
}

impl<V, K> Default for Scope<V, K>
where
    K: Hash + Eq + Clone + Debug,
{
    fn default() -> Self {
        Scope::new()
    }
}
impl<V, K> Frame<V, K>
where
    K: Hash + Eq + Clone + Debug,
{
    fn new(locals: Locals<V, K>, parent: Option<FramePtr<V, K>>) -> FramePtr<V, K> {
        Rc::new(RefCell::new(Frame {
            locals: locals,
            parent: parent,
        }))
    }

    fn get(&self, key: &K) -> Option<Rc<V>> {
        match (self.locals.get(key), &self.parent) {
            (Some(v), _) => Some(v.clone()),
            (None, &Some(ref p)) => p.borrow().get(key),
            (None, &None) => None,
        }
    }

    fn insert_at_root(&mut self, key: K, val: V) -> Option<Rc<V>> {
        match self.parent {
            None => self.locals.insert(key, Rc::new(val)),
            Some(ref ptr) => ptr.borrow_mut().insert_at_root(key, val),
        }
    }

    fn flatten(&self) -> HashMap<K, Rc<V>> {
        match &self.parent {
            &None => self.locals.clone(),
            &Some(ref p) => {
                let mut flattened = p.borrow().flatten();
                for (k, v) in &self.locals {
                    flattened.insert(k.clone(), v.clone());
                }
                flattened
            }
        }
    }

    fn map_clone<F, V2>(&self, f: &F) -> FramePtr<V2, K>
    where
        F: Fn(&V) -> V2,
    {
        let parent = match self.parent {
            None => None,
            Some(ref parent_frame_ptr) => Some((*parent_frame_ptr).borrow().map_clone(f).clone()),
        };
        let locals: Locals<V2, K> = self
            .locals
            .iter()
            .map(|(ref k, ref v)| {
                ((*k).clone(), Rc::new(/* */ f(v.as_ref()) /* */))
            })
            .collect();
        Frame::new(locals, parent)
    }
}

impl<K, V> FromIterator<(K, V)> for Scope<V, K>
where
    K: Hash + Eq + Debug + Clone,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(pairs: I) -> Scope<V, K> {
        Scope(Frame::new(
            pairs
                .into_iter()
                .map(|(key, val)| (key, Rc::new(val)))
                .collect(),
            None,
        ))
    }
}

#[test]
fn test_scope() {
    let mut root: Scope<usize, &'static str> = Scope::new();
    assert_eq!(root.insert("foo", 1), None);
    assert_eq!(root.insert("foo", 2), Some(Rc::new(1)));
    let mut child = root.new_child();
    assert_eq!(child.insert("foo", 3), None);
    assert_eq!(child.insert("bar", 1), None);
    assert_eq!(child.get(&"foo"), Some(Rc::new(3)));
    assert_eq!(child.get(&"bar"), Some(Rc::new(1)));
    assert_eq!(root.get(&"foo"), Some(Rc::new(2)));
    let child_foo = child.get(&"foo");
    drop(child);
    assert_eq!(child_foo, Some(Rc::new(3)));
}
