use crate::consts::*;
use crate::record::Record;

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub keys: [Option<Record>; MAX_KEYS],
    pub children: [Option<usize>; MAX_KEYS + 1],
    pub num_keys: usize,
    pub is_leaf: bool,
}

impl Node {
    pub fn new(is_leaf: bool) -> Self {
        Self {
            keys: [(); MAX_KEYS].map(|_| None),
            children: [(); MAX_KEYS + 1].map(|_| None),
            num_keys: 0,
            is_leaf,
        }
    }

    pub fn insert_non_full(&mut self, input: Record) -> bool {
        let i = self.num_keys;
        if i == MAX_KEYS {
            return false;
        }
        self.keys[i] = Some(input);
        self.num_keys += 1;
        return true;
    }

    /*

    pub fn is_full(&self) -> bool {
        self.keys.len() >= 2 * MIN_DEGREE - 1
    }

    pub fn search(&self, key: i32) -> Option<Record> {
        let mut i = 0;
        while i < self.keys.len() && key > self.keys[i].key {
            i += 1;
        }

        if i < self.keys.len() && key == self.keys[i].key {
            return Some(self.keys[i]);
        }

        if self.is_leaf {
            return None;
        }

        self.children[i].search(key)
    }

    pub fn insert_non_full(&mut self, rec: Record) {
        let mut i = self.keys.len();

        if self.is_leaf {
            self.keys.push(rec);
            i = self.keys.len() - 1;
            while i > 0 && self.keys[i] < self.keys[i - 1] {
                self.keys.swap(i, i - 1);
                i -= 1;
            }
        } else {
            while i > 0 && rec < self.keys[i - 1] {
                i -= 1;
            }

            if self.children[i].is_full() {
                self.split_child(i);
                if rec > self.keys[i] {
                    i += 1;
                }
            }
            self.children[i].insert_non_full(rec);
        }
    }

    pub fn split_child(&mut self, i: usize) {
        let t = MIN_DEGREE;
        let full_child = &mut self.children[i];
        let mut new_child = Box::new(Node::new(full_child.is_leaf));

        // Move the second half of keys to new child
        new_child.keys = full_child.keys.split_off(t);
        let median = full_child.keys.pop().unwrap();

        // Move the second half of children if not a leaf
        if !full_child.is_leaf {
            new_child.children = full_child.children.split_off(t);
        }

        // Insert median key and new child into parent
        self.keys.insert(i, median);
        self.children.insert(i + 1, new_child);
    }
    */
}
