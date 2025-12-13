use crate::config::MAX_KEYS;
use crate::node::*;
use crate::record::Record;
use crate::storage::Storage;

pub struct BPlusTree<S> {
    storage: S,
    root_loc: usize,
}

impl<S> BPlusTree<S>
where
    S: Storage,
{
    pub fn open(mut storage: S) -> Self {
        let root = Node::Leaf(LeafNode::new());
        storage.write_node(0, &root);
        BPlusTree {
            storage,
            root_loc: 0,
        }
    }

    pub fn find(&self, key: i32) -> Option<Record> {
        let mut current_loc = self.root_loc;
        loop {
            let node = self.storage.read_node(current_loc)?;
            match node {
                Node::Internal(internal) => {
                    let mut i = 0;
                    while i < internal.keys.len() && key >= internal.keys[i] {
                        i += 1;
                    }
                    current_loc = internal.children[i];
                }
                Node::Leaf(leaf) => {
                    for (i, k) in leaf.keys.iter().enumerate() {
                        if *k == key {
                            return Some(leaf.values[i]);
                        }
                    }
                    return None;
                }
            }
        }
    }

    pub fn insert(&mut self, value: Record) {
        let key = value[0];
        let mut path = Vec::new();
        let mut current_loc = self.root_loc;
        let mut current_node = self.storage.read_node(current_loc).unwrap();

        while let Node::Internal(internal) = current_node {
            path.push((current_loc, internal.clone()));
            let mut i = 0;
            while i < internal.keys.len() && key >= internal.keys[i] {
                i += 1;
            }
            current_loc = internal.children[i];
            current_node = self.storage.read_node(current_loc).unwrap();
        }

        // Insert into the leaf node
        if let Node::Leaf(mut leaf) = current_node {
            // Insert or update the key-value pair
            let mut inserted = false;
            for (i, k) in leaf.keys.iter_mut().enumerate() {
                if *k == key {
                    leaf.values[i] = value;
                    inserted = true;
                    break;
                } else if key < *k {
                    leaf.keys.insert(i, key);
                    leaf.values.insert(i, value);
                    inserted = true;
                    break;
                }
            }
            if !inserted {
                leaf.keys.push(key);
                leaf.values.push(value);
            }

            self.storage
                .write_node(current_loc, &Node::Leaf(leaf.clone()));

            if leaf.keys.len() > MAX_KEYS {
                self.split_leaf(current_loc, leaf, &mut path);
            }
        }
    }

    fn split_leaf(&mut self, loc: usize, leaf: LeafNode, path: &mut Vec<(usize, InternalNode)>) {
        let mid = leaf.keys.len() / 2;
        let new_leaf = LeafNode {
            keys: leaf.keys[mid..].to_vec(),
            values: leaf.values[mid..].to_vec(),
            next: leaf.next,
        };
        let new_leaf_loc = self.storage.total_nodes();

        let original_leaf = LeafNode {
            keys: leaf.keys[..mid].to_vec(),
            values: leaf.values[..mid].to_vec(),
            next: Some(new_leaf_loc),
        };

        self.storage.write_node(loc, &Node::Leaf(original_leaf));
        self.storage
            .write_node(new_leaf_loc, &Node::Leaf(new_leaf.clone()));

        let new_key = new_leaf.keys[0];
        if path.is_empty() {
            // Create a new root
            let new_root = Node::Internal(InternalNode {
                keys: vec![new_key],
                children: vec![loc, new_leaf_loc],
            });
            let new_root_loc = self.storage.total_nodes();
            self.storage.write_node(new_root_loc, &new_root);
            self.root_loc = new_root_loc;
        } else {
            self.insert_into_parent(new_key, new_leaf_loc, path);
        }
    }

    fn insert_into_parent(
        &mut self,
        key: i32,
        new_child_loc: usize,
        path: &mut Vec<(usize, InternalNode)>,
    ) {
        let (parent_loc, mut parent) = path.pop().unwrap();

        // Insert the new key and child into the parent
        let mut i = 0;
        while i < parent.keys.len() && key >= parent.keys[i] {
            i += 1;
        }
        parent.keys.insert(i, key);
        parent.children.insert(i + 1, new_child_loc);

        self.storage
            .write_node(parent_loc, &Node::Internal(parent.clone()));

        if parent.keys.len() > MAX_KEYS {
            self.split_internal(parent_loc, parent, path);
        }
    }

    fn split_internal(
        &mut self,
        loc: usize,
        internal: InternalNode,
        path: &mut Vec<(usize, InternalNode)>,
    ) {
        let mid = internal.keys.len() / 2;
        let new_internal = InternalNode {
            keys: internal.keys[mid + 1..].to_vec(),
            children: internal.children[mid + 1..].to_vec(),
        };
        let new_internal_loc = self.storage.total_nodes();

        let original_internal = InternalNode {
            keys: internal.keys[..mid].to_vec(),
            children: internal.children[..mid + 1].to_vec(),
        };

        self.storage
            .write_node(loc, &Node::Internal(original_internal));
        self.storage
            .write_node(new_internal_loc, &Node::Internal(new_internal));

        let new_key = internal.keys[mid];
        if path.is_empty() {
            // Create a new root
            let new_root = Node::Internal(InternalNode {
                keys: vec![new_key],
                children: vec![loc, new_internal_loc],
            });
            let new_root_loc = self.storage.total_nodes();
            self.storage.write_node(new_root_loc, &new_root);
            self.root_loc = new_root_loc;
        } else {
            self.insert_into_parent(new_key, new_internal_loc, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use crate::storage::InMemoryStorage;

    // Helper function to generate a large number of keys and values
    fn generate_large_dataset(size: usize) -> Vec<Record> {
        let keys: Vec<i32> = (1..=size as i32).collect();
        let values: Vec<Record> = keys.iter().map(|&k| [k; 7]).collect();
        values
    }

    // Helper function to generate random keys and values
    fn generate_random_dataset(size: usize) -> Vec<Record> {
        use rand::Rng;
        let mut rng = rand::rng();
        let keys: Vec<i32> = (0..size).map(|_| rng.random_range(1..10000)).collect();
        let values: Vec<Record> = keys.iter().map(|&k| [k; 7]).collect();
        values
    }

    #[test]
    fn test_initialization() {
        let storage = InMemoryStorage::new();
        let tree = BPlusTree::open(storage);
        assert_eq!(tree.root_loc, 0);
    }

    #[test]
    fn test_insert_and_find() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let value = [1, 1, 2, 3, 4, 5, 6]; // Example Record as [i32; 6]
        tree.insert(value);
        assert_eq!(tree.find(value[0]), Some(value));
    }

    #[test]
    fn test_multiple_inserts_and_finds() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let values = [
            [1, 1, 2, 3, 4, 5, 6],
            [2, 7, 8, 9, 10, 11, 12],
            [3, 13, 14, 15, 16, 17, 18],
            [4, 19, 20, 21, 22, 23, 24],
            [5, 25, 26, 27, 28, 29, 30],
        ];
        for rec in values {
            tree.insert(rec);
        }
        for rec in values {
            assert_eq!(tree.find(rec[0]), Some(rec));
        }
    }

    #[test]
    fn test_internal_split() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let values = [
            [1, 1, 2, 3, 4, 5, 6],
            [2, 7, 8, 9, 10, 11, 12],
            [3, 13, 14, 15, 16, 17, 18],
            [4, 19, 20, 21, 22, 23, 24],
            [5, 25, 26, 27, 28, 29, 30],
            [6, 31, 32, 33, 34, 35, 36],
            [7, 37, 38, 39, 40, 41, 42],
            [8, 43, 44, 45, 46, 47, 48],
            [9, 49, 50, 51, 52, 53, 54],
            [10, 55, 56, 57, 58, 59, 60],
            [11, 61, 62, 63, 64, 65, 66],
        ];
        for rec in values {
            tree.insert(rec);
        }
        for rec in values {
            assert_eq!(tree.find(rec[0]), Some(rec));
        }
    }

    #[test]
    fn test_update() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let key = 1;
        let initial_value = [key, 1, 2, 3, 4, 5, 6];
        let updated_value = [key, 7, 8, 9, 10, 11, 12];
        tree.insert(initial_value);
        assert_eq!(tree.find(key), Some(initial_value));
        tree.insert(updated_value);
        assert_eq!(tree.find(key), Some(updated_value));
    }

    #[test]
    fn test_non_existent_key() {
        let storage = InMemoryStorage::new();
        let tree = BPlusTree::open(storage);
        assert_eq!(tree.find(999), None); // Assuming 999 is not in the tree
    }

    #[test]
    fn test_out_of_order_inserts() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let values = [
            [5, 1, 2, 3, 4, 5, 6],
            [3, 7, 8, 9, 10, 11, 12],
            [1, 13, 14, 15, 16, 17, 18],
            [4, 19, 20, 21, 22, 23, 24],
            [2, 25, 26, 27, 28, 29, 30],
        ];
        for rec in values {
            tree.insert(rec);
        }
        for rec in values {
            assert_eq!(tree.find(rec[0]), Some(rec));
        }
    }

    #[test]
    fn test_large_number_of_inserts() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let num_keys = 100000;
        let values = generate_large_dataset(num_keys);
        for rec in &values {
            tree.insert(*rec);
        }
        for rec in &values {
            assert_eq!(tree.find(rec[0]), Some(*rec));
        }
    }

    #[test]
    fn test_boundary_values() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);

        let min_value = [i32::MIN; 7];
        let max_value = [i32::MAX; 7];

        tree.insert(min_value);
        tree.insert(max_value);

        assert_eq!(tree.find(min_value[0]), Some(min_value));
        assert_eq!(tree.find(max_value[0]), Some(max_value));
    }

    #[test]
    fn test_random_insertions() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);
        let values = generate_random_dataset(10000);

        for rec in &values {
            tree.insert(*rec);
        }
        for rec in &values {
            assert_eq!(tree.find(rec[0]), Some(*rec));
        }
    }
}
