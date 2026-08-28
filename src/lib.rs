//! Rust implementation of Wormhole; based off of the EuroSys paper.
//! (Wu, Ni, Jiang; EuroSys 2019).
//!
//! https://dl.acm.org/doi/epdf/10.1145/3302424.3303955
//!
//! Author: Akrm Al-Hakimi

impl<V> Wormholes<V> {
    pub fn new() -> Self {
        Wormholes {
            leaves: vec![Leaf::new()],
            len: 0,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&V> {
        let leaf = &self.leaves[self.target_leaf(key)];
        let i = leaf
            .entries
            .binary_search_by(|(k, _)| k.as_slice().cmp(key))
            .ok()?;
        Some(&leaf.entries[i].1)
    }

    pub fn insert(&mut self, key: &[u8], value: V) -> Option<V> {
        let li = self.target_leaf(key);
        let leaf = &mut self.leaves[li];
        match leaf
            .entries
            .binary_search_by(|(k, _)| k.as_slice().cmp(key))
        {
            Ok(i) => Some(std::mem::replace(&mut leaf.entries[i].1, value)),
            Err(i) => {
                leaf.entries.insert(i, (key.to_vec(), value));
                self.len += 1;
                if leaf.entries.len() > MAX_ENTRIES {
                    self.split(li);
                }
                None
            }
        }
    }

    /// Naive middle split. When anchors are implemented,
    /// the split point will be chosen to minimize anchor length.
    /// Section 2.2, Algorithm 4
    fn split(&mut self, li: usize) {
        let leaf = &mut self.leaves[li];
        let right_entries = leaf.entries.split_off(leaf.entries.len() / 2);
        self.leaves.insert(
            li + 1,
            Leaf {
                entries: right_entries,
            },
        );
    }

    /// Ascending iteration over all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &V)> {
        self.leaves
            .iter()
            .flat_map(|leaf| leaf.entries.iter().map(|(k, v)| (k.as_slice(), v)))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Index of leaf whose key range covers `key`.
    /// Eventually, MetaTrieHT will replace this.
    fn target_leaf(&self, key: &[u8]) -> usize {
        // partition_point is the first leaf whose
        // min_key > key, then step back one.
        // An empty first leaf routes everything
        // to index 0.
        let idx = self
            .leaves
            .partition_point(|leaf| !leaf.entries.is_empty() && leaf.min_key() <= key);
        idx.saturating_sub(1)
    }

    /// Debug-only invariant check, called by tests after mutations.
    #[cfg(test)]
    fn check_invariants(&self) {
        assert!(!self.leaves.is_empty(), "leaf list never empty");
        let mut prev: Option<&[u8]> = None;
        let mut counted = 0;
        for (i, leaf) in self.leaves.iter().enumerate() {
            assert!(
                i == 0 || !leaf.entries.is_empty(),
                "only the first leaf may be empty"
            );
            assert!(leaf.entries.len() <= MAX_ENTRIES, "leaf over capacity");
            for (k, _) in &leaf.entries {
                if let Some(p) = prev {
                    assert!(p < k.as_slice(), "keys out of order");
                }
                prev = Some(k);
                counted += 1;
            }
        }
        assert_eq!(counted, self.len, "len out of sync");
    }
}

impl<V> Default for Wormholes<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// An ordered map from byte-string keys to values.
pub struct Wormholes<V> {
    /// Sorted, non-overlapping leaves
    /// Only the very first leaf may be empty
    /// All keys in leaves[i] are strictly
    /// less than all keys in leaves[i + 1]
    leaves: Vec<Leaf<V>>,
    len: usize,
}

impl<V> Leaf<V> {
    fn new() -> Self {
        Leaf {
            entries: Vec::new(),
        }
    }

    /// Smallest key in this leaf
    /// This is only valid for non-empty leaves.
    fn min_key(&self) -> &[u8] {
        &self.entries[0].0
    }
}

/// One node of the LeafList: a sorted run of (K, V) entries
struct Leaf<V> {
    entries: Vec<(Vec<u8>, V)>,
}

/// Max entries per leaf before it splits
/// The paper uses 128 so I guess we'll use that
const MAX_ENTRIES: usize = 128;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_roundtrip() {
        let mut wh = Wormholes::new();
        assert_eq!(wh.insert(b"cat", 1), None);
        assert_eq!(wh.insert(b"car", 2), None);
        assert_eq!(wh.insert(b"cat", 3), Some(1));
        assert_eq!(wh.get(b"cat"), Some(&3));
        assert_eq!(wh.get(b"car"), Some(&2));
        assert_eq!(wh.get(b"dog"), None);
        assert_eq!(wh.len(), 2);
        wh.check_invariants();
    }

    #[test]
    fn split_keeps_order() {
        let mut wh = Wormholes::new();
        for i in 0..1000u32 {
            // insertion order is scrambled on purpose
            let k = (i.wrapping_mul(2654435761)).to_be_bytes();
            wh.insert(&k, i);
            wh.check_invariants();
        }
        assert_eq!(wh.len(), 1000);
        assert!(wh.leaves.len() > 1, "should have split at least once");
        let keys: Vec<_> = wh.iter().map(|(k, _)| k.to_vec()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "iteration must be ascending");
    }
}
