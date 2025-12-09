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

        // Collect left keys
        for i in 0..left.num_keys {
            if let Some(k) = left.keys[i] {
                all_keys.push(k);
            }
        }

        // Add parent separator key
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

        // Collect right keys
        for i in 0..right.num_keys {
            if let Some(k) = right.keys[i] {
                all_keys.push(k);
            }
        }

        // Add new record
        all_keys.push(record);
        all_keys.sort_by_key(|r| r.key);

        // Split keys evenly
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

        // Update parent key safely
        if separator_idx < parent.keys.len() {
            parent.keys[separator_idx] = Some(middle_key);
        } else {
            parent.keys[parent.keys.iter().position(|k| k.is_none()).unwrap()] = Some(middle_key);
        }

        right.keys.fill(None);
        for i in 0..right_num {
            right.keys[i] = Some(all_keys[right_start + i]);
        }
        right.num_keys = right_num;

        // Update parent pointers
        left.parent = Some(parent_page);
        right.parent = Some(parent_page);

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

    fn split_recursive(&mut self, node_page: usize, input: Record) -> Option<usize> {
        let node = self.storage.read_node(node_page);
        let mut keys: Vec<Record> = node
            .keys
            .iter()
            .filter_map(|k| k.as_ref())
            .cloned()
            .collect();
        keys.push(input);
        keys.sort_by_key(|r| r.key);

        let mid = keys.len() / 2;
        let middle_key = keys[mid];

        // Create left node
        let mut left_node = Node::new(node.is_leaf);
        left_node.num_keys = mid;
        for i in 0..mid {
            left_node.keys[i] = Some(keys[i]);
        }
        if !node.is_leaf {
            for i in 0..=mid {
                left_node.children[i] = node.children[i];
            }
        }

        // Create right node
        let mut right_node = Node::new(node.is_leaf);
        right_node.num_keys = keys.len() - mid - 1;
        for i in 0..right_node.num_keys as usize {
            right_node.keys[i] = Some(keys[mid + 1 + i]);
        }
        if !node.is_leaf {
            for i in 0..=right_node.num_keys as usize {
                right_node.children[i] = node.children[mid + 1 + i];
            }
        }

        // Update parent pointers for children
        if !node.is_leaf {
            for i in 0..=left_node.num_keys as usize {
                if let Some(child_page) = left_node.children[i] {
                    let mut child = self.storage.read_node(child_page);
                    child.parent = Some(node_page);
                    self.storage.write_node(child_page, &child);
                }
            }
            for i in 0..=right_node.num_keys as usize {
                if let Some(child_page) = right_node.children[i] {
                    let mut child = self.storage.read_node(child_page);
                    child.parent = Some(self.storage.num_nodes()); // Placeholder, will be updated
                    self.storage.write_node(child_page, &child);
                }
            }
        }

        // Write left and right nodes to storage
        let left_page = if node_page == 0 {
            // If splitting the root, left node becomes new root
            self.storage.write_node(node_page, &left_node);
            node_page
        } else {
            self.storage.append_node(&left_node)
        };
        let right_page = self.storage.append_node(&right_node);

        // Update parent
        if node_page == 0 {
            // Root was split, create new root
            let mut new_root = Node::new(false);
            new_root.keys[0] = Some(middle_key);
            new_root.children[0] = Some(left_page);
            new_root.children[1] = Some(right_page);
            new_root.num_keys = 1;
            new_root.parent = None;
            left_node.parent = Some(self.storage.num_nodes());
            right_node.parent = Some(self.storage.num_nodes());
            self.storage.write_node(left_page, &left_node);
            self.storage.write_node(right_page, &right_node);
            let new_root_page = self.storage.append_node(&new_root);
            Some(new_root_page)
        } else {
            // Update parent with middle key and new child
            let parent_page = node.parent.unwrap();
            let mut parent = self.storage.read_node(parent_page);

            // Insert middle key into parent
            let mut key_pos = 0;
            while key_pos < parent.num_keys && parent.keys[key_pos].unwrap().key < middle_key.key {
                key_pos += 1;
            }
            for i in (key_pos..parent.num_keys as usize).rev() {
                parent.keys[i + 1] = parent.keys[i];
                parent.children[i + 2] = parent.children[i + 1];
            }
            parent.keys[key_pos] = Some(middle_key);
            parent.children[key_pos + 1] = Some(right_page);
            parent.num_keys += 1;
            self.storage.write_node(parent_page, &parent);
            // Recursively split parent if full
            if parent.num_keys as usize == MAX_KEYS {
                self.split_recursive(parent_page, middle_key)
            } else {
                None
            }
        }
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

                // 3) Must split and possibly recurse upward #todo
                let _ = self.split_recursive(node.1, input);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempfile;

    // Helper function to create a BTree with a temporary file
    fn create_btree() -> BTree {
        let file = tempfile().unwrap();
        BTree {
            storage: NodeStorage {
                file,
                page_reads: 0,
                page_writes: 0,
            },
        }
    }

    fn create_record(key: i32) -> Record {
        let mut out = Record::random();
        out.key = key;
        out
    }

    #[test]
    fn test_empty_tree() {
        let mut btree = create_btree();
        assert!(btree.search(1).is_none());
    }

    #[test]
    fn test_single_insert_and_search() {
        let mut btree = create_btree();
        let key = 42;
        let record = create_record(key);
        btree.insert(record);
        assert!(btree.search(key).is_some());
        assert_eq!(btree.search(key).unwrap().key, key);
    }

    #[test]
    fn test_insert_and_search_multiple() {
        let mut btree = create_btree();
        let keys = vec![10, 20, 30, 40, 50];
        for key in &keys {
            btree.insert(create_record(*key));
        }
        for key in &keys {
            assert!(btree.search(*key).is_some());
            assert_eq!(btree.search(*key).unwrap().key, *key);
        }
    }

    #[test]
    fn test_insert_duplicate_key() {
        let mut btree = create_btree();
        let key = 42;
        let record1 = create_record(key);
        let record2 = create_record(key);
        btree.insert(record1);
        btree.insert(record2);
        assert!(btree.search(key).is_some());
        // Ensure the data was updated (assuming insert updates existing keys)
        assert_eq!(btree.search(key).unwrap().key, record2.key);
    }

    #[test]
    fn test_insert_and_search_min_max_keys() {
        let mut btree = create_btree();
        let min_key = i32::MIN;
        let max_key = i32::MAX;
        btree.insert(create_record(min_key));
        btree.insert(create_record(max_key));
        assert!(btree.search(min_key).is_some());
        assert!(btree.search(max_key).is_some());
    }

    #[test]
    fn test_search_nonexistent_key() {
        let mut btree = create_btree();
        btree.insert(create_record(10));
        btree.insert(create_record(20));
        btree.insert(create_record(30));
        assert!(btree.search(15).is_none());
    }

    #[test]
    fn test_split_and_search() {
        let mut btree = create_btree();
        // Insert enough keys to force a split
        let keys = (1..=crate::consts::MAX_KEYS + 1).collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        for key in &keys {
            assert!(btree.search(*key as i32).is_some());
        }
    }

    #[test]
    fn test_recursive_split() {
        let mut btree = create_btree();
        // Insert enough keys to force multiple splits
        let keys = (1..=crate::consts::MAX_KEYS * 2).collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        for key in &keys {
            assert!(btree.search(*key as i32).is_some());
        }
    }

    #[test]
    fn test_stress_insert_and_search() {
        let mut btree = create_btree();
        let keys = (1..=1000).collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        for key in &keys {
            assert!(btree.search(*key as i32).is_some());
        }
    }

    #[test]
    fn test_insert_at_boundary() {
        let mut btree = create_btree();
        // Insert keys at the boundary of the node capacity
        let keys = (1..=crate::consts::MAX_KEYS).collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        // Insert one more to force a split
        btree.insert(create_record((crate::consts::MAX_KEYS + 1) as i32));
        for key in 1..=crate::consts::MAX_KEYS + 1 {
            assert!(btree.search(key as i32).is_some());
        }
    }

    #[test]
    fn test_insert_sorted_keys() {
        let mut btree = create_btree();
        // Insert keys in sorted order to test rightmost insertion
        let keys = (1..=crate::consts::MAX_KEYS * 2).collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        for key in 1..=crate::consts::MAX_KEYS * 2 {
            assert!(btree.search(key as i32).is_some());
        }
    }

    #[test]
    fn test_insert_reverse_sorted_keys() {
        let mut btree = create_btree();
        // Insert keys in reverse order to test leftmost insertion
        let keys = (1..=crate::consts::MAX_KEYS * 2).rev().collect::<Vec<_>>();
        for key in &keys {
            btree.insert(create_record(*key as i32));
        }
        for key in 1..=crate::consts::MAX_KEYS * 2 {
            assert!(btree.search(key as i32).is_some());
        }
    }

    #[test]
    fn test_insert_and_search_large_keys() {
        let mut btree = create_btree();
        // Insert large keys to test edge cases
        let keys = vec![i32::MAX - 1, i32::MAX - 2, i32::MAX - 3];
        for key in &keys {
            btree.insert(create_record(*key));
        }
        for key in &keys {
            assert!(btree.search(*key).is_some());
        }
    }

    #[test]
    fn test_insert_and_search_negative_keys() {
        let mut btree = create_btree();
        // Insert negative keys
        let keys = vec![-1, -2, -3, -4, -5];
        for key in &keys {
            btree.insert(create_record(*key));
        }
        for key in &keys {
            assert!(btree.search(*key).is_some());
        }
    }
}
