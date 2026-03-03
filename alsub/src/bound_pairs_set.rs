use im_rc::HashMap;
use im_rc::HashSet;

use crate::core::VarSpec;
use crate::parse_types::SourceLoc;

#[derive(Debug, Clone, Default)]
pub struct BoundPairsSet {
    map: HashMap<SourceLoc, SourceLoc>,
    flipped: bool,
}
impl BoundPairsSet {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn filter_left(&mut self, f: impl Fn(SourceLoc) -> bool) {
        if self.flipped {
            self.map.retain(|_, v| f(*v));
        } else {
            self.map.retain(|k, _| f(*k));
        }
    }

    pub fn filter_right(&mut self, f: impl Fn(SourceLoc) -> bool) {
        if self.flipped {
            self.map.retain(|k, _| f(*k));
        } else {
            self.map.retain(|_, v| f(*v));
        }
    }

    pub fn push(&mut self, mut pair: (SourceLoc, SourceLoc)) {
        if self.flipped {
            pair = (pair.1, pair.0);
        }
        self.map.insert(pair.0, pair.1);
    }

    pub fn flip(&self) -> Self {
        Self {
            map: self.map.clone(),
            flipped: !self.flipped,
        }
    }

    pub fn update_intersect(&mut self, other: &Self) -> bool {
        if self.map.is_empty() {
            return false;
        }

        if other.map.is_empty() {
            self.clear();
            return true;
        }

        let same_orientation = self.flipped == other.flipped;
        let old_len = self.map.len();
        self.map.retain(|k, v| {
            if same_orientation {
                other.map.get(k).copied() == Some(*v)
            } else {
                other.map.get(v).copied() == Some(*k)
            }
        });
        self.map.len() != old_len
    }

    pub fn get(&self, loc1: SourceLoc, loc2: SourceLoc) -> bool {
        if self.flipped {
            self.map.get(&loc2).copied() == Some(loc1)
        } else {
            self.map.get(&loc1).copied() == Some(loc2)
        }
    }

    // Return true if there's any (loc, loc2) in self such that (loc, name) is in lhs and (loc2, name) is in rhs
    pub fn disjoint_union_vars_have_match<'a>(&self, mut lhs: &'a HashSet<VarSpec>, mut rhs: &'a HashSet<VarSpec>) -> bool {
        if self.flipped {
            (lhs, rhs) = (rhs, lhs);
        }

        for spec in lhs.iter().copied() {
            if let Some(loc2) = self.map.get(&spec.loc).copied() {
                let expect = VarSpec {
                    loc: loc2,
                    name: spec.name,
                };
                if rhs.contains(&expect) {
                    return true;
                }
            }
        }

        false
    }
}
