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

    pub fn find(&mut self, key: i32) -> Option<Record> {
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
    use crate::storage::InMemoryStorage;

    use proptest::prelude::*;
    use proptest::proptest;

    use std::collections::BTreeMap;

    fn arb_record() -> impl Strategy<Value = Record> {
        prop::array::uniform7(any::<i32>())
    }

    fn arb_key() -> impl Strategy<Value = i32> {
        any::<i32>()
    }

    #[derive(Debug, Clone)]
    enum Op {
        Insert(Record),
        Find(i32),
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        prop_oneof![
            arb_record().prop_map(Op::Insert),
            arb_key().prop_map(Op::Find),
        ]
    }

    fn arb_ops() -> impl Strategy<Value = Vec<Op>> {
        // Long enough to trigger many splits, but still fast
        prop::collection::vec(arb_op(), 1..500)
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 200,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_bplustree_matches_btreemap(ops in arb_ops()) {
            let storage = InMemoryStorage::new();
            let mut tree = BPlusTree::open(storage);

            let mut model = BTreeMap::<i32, Record>::new();

            for op in ops {
                match op {
                    Op::Insert(record) => {
                        let key = record[0];
                        tree.insert(record);
                        model.insert(key, record);
                    }

                    Op::Find(key) => {
                        let tree_res = tree.find(key);
                        let model_res = model.get(&key).copied();
                        prop_assert_eq!(tree_res, model_res);
                    }
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_all_inserted_keys_are_findable(records in prop::collection::vec(arb_record(), 0..1000)) {
            let storage = InMemoryStorage::new();
            let mut tree = BPlusTree::open(storage);
            let mut model = BTreeMap::<i32, Record>::new();

            for rec in records {
                tree.insert(rec);
                model.insert(rec[0], rec);
            }

            for (key, value) in model {
                prop_assert_eq!(tree.find(key), Some(value));
            }
        }
    }

    #[test]
    fn sanity_single_insert() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);

        let rec = [42, 1, 2, 3, 4, 5, 6];
        tree.insert(rec);

        assert_eq!(tree.find(42), Some(rec));
        assert_eq!(tree.find(7), None);
    }

    #[test]
    fn sanity_update_overwrites_value() {
        let storage = InMemoryStorage::new();
        let mut tree = BPlusTree::open(storage);

        let r1 = [1, 1, 1, 1, 1, 1, 1];
        let r2 = [1, 9, 9, 9, 9, 9, 9];

        tree.insert(r1);
        tree.insert(r2);

        assert_eq!(tree.find(1), Some(r2));
    }
}
