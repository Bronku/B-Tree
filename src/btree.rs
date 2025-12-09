use crate::Record;
use crate::consts::MAX_KEYS;
use crate::node::Node;
use crate::node_storage::NodeStorage;

#[derive(Debug)]
pub struct BTree {
    storage: NodeStorage,
}

enum FindResult {
    EmptyTree,

    Found {
        page: usize,
        index: usize,
        node: Node,
        record: Record,
    },

    NotFound {
        node: (Node, usize),           // leaf node where insertion must happen
        parent: Option<(Node, usize)>, // optional parent for reducing disk reads on split/compensate
    },
}

impl BTree {
    pub fn new(filename: &str) -> Self {
        BTree {
            storage: NodeStorage::open(filename),
        }
    }

    fn find(&mut self, key: i32) -> FindResult {
        if self.storage.num_nodes() == 0 {
            return FindResult::EmptyTree;
        }

        let mut current = 0;
        let mut parent: Option<(Node, usize)> = None;

        'outer: loop {
            let node = self.storage.read_node(current);

            if node.is_leaf {
                for i in 0..node.num_keys {
                    if let Some(rec) = node.keys[i] {
                        if rec.key == key {
                            return FindResult::Found {
                                page: current,
                                index: i,
                                node,
                                record: rec,
                            };
                        }
                    }
                }
                return FindResult::NotFound {
                    node: (node, current),
                    parent,
                };
            }

            let mut prev_key = i32::MIN;

            for i in 0..node.num_keys {
                let node_key = node.keys[i].unwrap().key;

                if key == node_key {
                    let rec = node.keys[i].unwrap();
                    return FindResult::Found {
                        page: current,
                        index: i,
                        node,
                        record: rec,
                    };
                }

                if prev_key < key && key < node_key {
                    parent = Some((node, current));
                    current = node.children[i].unwrap();
                    continue 'outer;
                }

                prev_key = node_key;
            }

            parent = Some((node, current));
            current = node.children[node.num_keys].unwrap();
        }
    }

    pub fn search(&mut self, key: i32) -> Option<Record> {
        match self.find(key) {
            FindResult::Found { record, .. } => Some(record),
            _ => None,
        }
    }

    fn try_insert_without_split(mut node: Node, key: Record) -> Option<Node> {
        if node.num_keys >= MAX_KEYS {
            return None;
        }

        let mut pos = 0;
        while pos < node.num_keys && node.keys[pos].unwrap().key < key.key {
            pos += 1;
        }

        for i in (pos..node.num_keys).rev() {
            node.keys[i + 1] = node.keys[i];
        }

        node.keys[pos] = Some(key);
        node.num_keys += 1;
        Some(node)
    }

    /// Helper method to redistribute keys across two siblings and parent
    fn redistribute(
        mut left: Node,
        left_page: usize,
        mut parent: Node,
        parent_page: usize,
        mut right: Node,
        right_page: usize,
        record: Record,
    ) -> Vec<(Node, usize)> {
        let mut all_keys: Vec<Record> = vec![];

        for i in 0..left.num_keys {
            if let Some(k) = left.keys[i] {
                all_keys.push(k);
            }
        }

        let last_left_key = all_keys.last().unwrap().key;
        let separator_idx = parent
            .keys
            .iter()
            .position(|k| match k {
                Some(r) => r.key > last_left_key,
                None => false,
            })
            .unwrap();
        let separating_key = parent.keys[separator_idx].unwrap();

        all_keys.push(separating_key);

        for i in 0..right.num_keys {
            if let Some(k) = right.keys[i] {
                all_keys.push(k);
            }
        }

        all_keys.push(record);
        all_keys.sort_by_key(|r| r.key);

        let total_keys = all_keys.len();
        let left_num = total_keys / 2;
        let right_num = total_keys - left_num - 1;

        left.keys.fill(None);
        for i in 0..left_num {
            left.keys[i] = Some(all_keys[i]);
        }
        left.num_keys = left_num;

        let middle_key = all_keys[left_num];
        let right_start = left_num + 1;

        let parent_idx = parent
            .children
            .iter()
            .position(|&c| c == Some(right_page))
            .unwrap();
        parent.keys[parent_idx - 1] = Some(middle_key);

        right.keys.fill(None);
        for i in 0..right_num {
            right.keys[i] = Some(all_keys[right_start + i]);
        }
        right.num_keys = right_num;

        vec![
            (left, left_page),
            (parent, parent_page),
            (right, right_page),
        ]
    }

    fn try_compensate(
        &mut self,
        node: (Node, usize),
        parent: Option<(Node, usize)>,
        input: Record,
    ) -> Option<Vec<(Node, usize)>> {
        let parent_idx = parent?.1;
        let parent = parent?.0;
        let node_idx = node.1;
        let node = node.0;

        let node_idx_in_parent = parent
            .children
            .iter()
            .position(|&child_opt| child_opt == Some(node_idx))?;

        if node_idx_in_parent > 0 {
            let left_sibling_idx = parent.children[node_idx_in_parent - 1]?;
            let left_sibling = self.storage.read_node(left_sibling_idx);

            if left_sibling.is_leaf && left_sibling.num_keys < MAX_KEYS {
                return Some(BTree::redistribute(
                    left_sibling,
                    left_sibling_idx,
                    parent,
                    parent_idx,
                    node,
                    node_idx,
                    input,
                ));
            }
        }

        // 4. Attempt right sibling compensation
        if node_idx_in_parent + 1 < parent.children.len() {
            let right_sibling_idx = parent.children[node_idx_in_parent + 1]?;
            let right_sibling = self.storage.read_node(right_sibling_idx);

            if right_sibling.is_leaf && right_sibling.num_keys < MAX_KEYS {
                return Some(BTree::redistribute(
                    node,
                    node_idx,
                    parent,
                    parent_idx,
                    right_sibling,
                    right_sibling_idx,
                    input,
                ));
            }
        }

        None
    }

    pub fn insert(&mut self, input: Record) {
        use FindResult::*;

        match self.find(input.key) {
            Found {
                page,
                mut node,
                index,
                ..
            } => {
                // Key exists ─ update record
                node.keys[index] = Some(input);
                self.storage.write_node(page, &node);
                return;
            }

            EmptyTree => {
                // Create first root
                let mut root = Node::new(true);
                root.keys[0] = Some(input);
                root.num_keys = 1;
                root.parent = None;
                self.storage.append_node(&root);
                return;
            }

            NotFound { node, parent } => {
                // Main insertion flow
                let key = input;

                // 1) Try normal insertion
                if let Some(updated_node) = BTree::try_insert_without_split(node.0, input) {
                    self.storage.write_node(node.1, &updated_node);
                    return;
                }

                // 2) Try compensation
                if let Some(updated_nodes) = self.try_compensate(node, parent, input) {
                    for (n, idx) in updated_nodes {
                        self.storage.write_node(idx, &n);
                    }
                    return;
                }

                // 3) Must split and possibly recurse upward
                let nodes_to_write = BTree::split(node.0, parent, input);
                for (n, idx) in nodes_to_write {
                    self.storage.write_node(idx, &n);
                }
            }
        }
    }
}
