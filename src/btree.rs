use crate::node::*;
use crate::record::Record;
use crate::storage::Storage;
const MAX_KEYS: usize = 4;

pub struct BPlusTree<S> {
    storage: S,
    root_loc: usize,
}

impl<S> BPlusTree<S>
where
    S: Storage,
{
    pub fn open(mut storage: S) -> Self {
        // Initialize with an empty root node (leaf)
        let root = Node::Leaf(LeafNode {
            keys: Vec::new(),
            values: Vec::new(),
            next: None,
        });
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

    pub fn insert(&mut self, key: i32, value: Record) {
        let mut path = Vec::new();
        let mut current_loc = self.root_loc;
        let mut current_node = self.storage.read_node(current_loc).unwrap();

        // Traverse to the leaf node, recording the path
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

            // Write the updated leaf back to storage
            self.storage
                .write_node(current_loc, &Node::Leaf(leaf.clone()));

            // Check if the leaf needs to be split
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
