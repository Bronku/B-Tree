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
        let (parent, parent_idx) = parent?;
        let (node, node_idx) = node;

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

    fn split_node(
        node_tuple: (Node, usize),
        parent: Option<(Node, usize)>,
        input: Record,
        next_node_idx: usize,
    ) -> Vec<(Node, usize)> {
        let (mut node, node_idx) = node_tuple;

        // 1. Collect all keys (node + input)
        let mut all_keys: Vec<Record> = (0..node.num_keys).filter_map(|i| node.keys[i]).collect();
        all_keys.push(input);
        all_keys.sort_by_key(|r| r.key);

        // 2. Determine middle key
        let mid_idx = all_keys.len() / 2;
        let middle_key = all_keys[mid_idx];

        // 3. Update left node with keys before middle
        node.keys.fill(None);
        for i in 0..mid_idx {
            node.keys[i] = Some(all_keys[i]);
        }
        node.num_keys = mid_idx;

        // 4. Create new right node
        let mut right = Node::new(node.is_leaf);
        for i in 0..(all_keys.len() - mid_idx - 1) {
            right.keys[i] = Some(all_keys[mid_idx + 1 + i]);
        }
        right.num_keys = all_keys.len() - mid_idx - 1;
        let right_idx = next_node_idx;

        let mut result = vec![(node, node_idx), (right, right_idx)];

        // 5. Update or create parent
        match parent {
            Some((mut par_node, par_idx)) => {
                let mut insert_pos = 0;
                while insert_pos < par_node.num_keys
                    && par_node.keys[insert_pos].unwrap().key < middle_key.key
                {
                    insert_pos += 1;
                }

                for i in (insert_pos..par_node.num_keys).rev() {
                    par_node.keys[i + 1] = par_node.keys[i];
                    par_node.children[i + 2] = par_node.children[i + 1];
                }

                par_node.keys[insert_pos] = Some(middle_key);
                par_node.children[insert_pos + 1] = Some(right_idx);
                par_node.num_keys += 1;

                result.push((par_node, par_idx));
            }
            None => {
                let mut root = Node::new(false);
                root.keys[0] = Some(middle_key);
                root.num_keys = 1;
                root.children[0] = Some(node_idx);
                root.children[1] = Some(right_idx);

                // Assign root index — usually 0 if creating from empty tree
                result.push((root, 0));
            }
        }

        result
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
                let nodes_to_write =
                    BTree::split_node(node, parent, input, self.storage.num_nodes());
                for (n, idx) in nodes_to_write {
                    self.storage.write_node(idx, &n);
                }
            }
        }
    }
}
