use im_rc::HashMap;
use std::hash::Hash;

// utility functions
#[allow(unused)]
pub fn sorted<T: Ord>(it: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v = it.into_iter().collect::<Vec<_>>();
    v.sort_unstable();
    v
}

pub struct UnwindPoint(usize);
pub struct UnwindMap<K: Eq + Hash + Clone, V: Clone> {
    pub m: HashMap<K, V>,
    snapshots: Vec<HashMap<K, V>>,
}
impl<K: Eq + Hash + Clone, V: Clone> UnwindMap<K, V> {
    pub fn new() -> Self {
        Self {
            m: HashMap::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.m.get(k)
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.m.insert(k, v);
    }

    pub fn unwind_point(&mut self) -> UnwindPoint {
        let idx = self.snapshots.len();
        self.snapshots.push(self.m.clone());
        UnwindPoint(idx)
    }

    pub fn unwind(&mut self, n: UnwindPoint) {
        self.m = self.snapshots[n.0].clone();
        self.snapshots.truncate(n.0);
    }

    pub fn make_permanent(&mut self, n: UnwindPoint) {
        assert!(n.0 == 0);
        self.snapshots.clear();
    }
}
