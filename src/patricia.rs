fn bit_at(key: &[u8], i: usize) -> bool {
    let byte = i / 8;
    if byte < key.len() {
        key[byte] & (0x80 >> (i % 8)) != 0
    } else {
        i == key.len() * 8
    }
}

fn first_diff_bit(a: &[u8], b: &[u8]) -> Option<usize> {
    let shared = a.len().min(b.len());
    for i in 0..shared {
        if a[i] != b[i] {
            return Some(i * 8 + (a[i] ^ b[i]).leading_zeros() as usize);
        }
    }
    if a.len() == b.len() {
        return None;
    }
    let end = a.len().max(b.len()) * 8;
    (shared * 8..=end).find(|&i| bit_at(a, i) != bit_at(b, i))
}

#[derive(Debug)]
enum Node<V> {
    Leaf { key: Vec<u8>, value: V },
    Internal { bit: usize, zero: usize, one: usize },
}

/// A map from byte-string keys to values, backed by a binary Patricia trie.
///
/// Nodes live in a single arena and are referred to by index, so the tree
/// carries no per-node allocation beyond the key bytes themselves.
#[derive(Debug)]
pub struct PatriciaTrie<V> {
    nodes: Vec<Node<V>>,
    root: Option<usize>,
    len: usize,
}

impl<V> Default for PatriciaTrie<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> PatriciaTrie<V> {
    pub fn new() -> Self {
        PatriciaTrie {
            nodes: Vec::new(),
            root: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, node: Node<V>) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn descend(&self, start: usize, key: &[u8]) -> usize {
        let mut cur = start;
        while let Node::Internal { bit, zero, one } = self.nodes[cur] {
            cur = if bit_at(key, bit) { one } else { zero };
        }
        cur
    }

    pub fn insert(&mut self, key: &[u8], value: V) -> Option<V> {
        let root = match self.root {
            None => {
                let leaf = self.push(Node::Leaf {
                    key: key.to_vec(),
                    value,
                });
                self.root = Some(leaf);
                self.len = 1;
                return None;
            }
            Some(root) => root,
        };

        let candidate = self.descend(root, key);
        let diff = match &mut self.nodes[candidate] {
            Node::Leaf {
                key: existing,
                value: slot,
            } => match first_diff_bit(existing, key) {
                None => return Some(std::mem::replace(slot, value)),
                Some(diff) => diff,
            },
            Node::Internal { .. } => unreachable!("descend always ends at a leaf"),
        };

        let mut parent: Option<(usize, bool)> = None;
        let mut cur = root;
        while let Node::Internal { bit, zero, one } = self.nodes[cur] {
            if bit >= diff {
                break;
            }
            let go_one = bit_at(key, bit);
            parent = Some((cur, go_one));
            cur = if go_one { one } else { zero };
        }

        let leaf = self.push(Node::Leaf {
            key: key.to_vec(),
            value,
        });
        let branch = if bit_at(key, diff) {
            Node::Internal {
                bit: diff,
                zero: cur,
                one: leaf,
            }
        } else {
            Node::Internal {
                bit: diff,
                zero: leaf,
                one: cur,
            }
        };
        let branch = self.push(branch);

        match parent {
            None => self.root = Some(branch),
            Some((parent, went_one)) => match &mut self.nodes[parent] {
                Node::Internal { zero, one, .. } => {
                    *(if went_one { one } else { zero }) = branch;
                }
                Node::Leaf { .. } => unreachable!("parent is always internal"),
            },
        }

        self.len += 1;
        None
    }

    pub fn get(&self, key: &[u8]) -> Option<&V> {
        let leaf = self.descend(self.root?, key);
        match &self.nodes[leaf] {
            Node::Leaf {
                key: existing,
                value,
            } if existing.as_slice() == key => Some(value),
            _ => None,
        }
    }
}
