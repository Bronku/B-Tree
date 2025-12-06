const MIN_DEGREE: usize = 3; // Minimum degree (minimum children per node)

#[derive(Debug)]
pub struct BTree<K: Ord> {
    root: Option<Box<Node<K>>>,
}

#[derive(Debug)]
struct Node<K: Ord> {
    keys: Vec<K>,
    children: Vec<Box<Node<K>>>,
    is_leaf: bool,
}

impl<K: Ord> Node<K> {
    fn new(is_leaf: bool) -> Self {
        Node {
            keys: Vec::new(),
            children: Vec::new(),
            is_leaf,
        }
    }

    fn is_full(&self) -> bool {
        self.keys.len() >= 2 * MIN_DEGREE - 1
    }

    fn search(&self, key: &K) -> bool {
        let mut i = 0;
        while i < self.keys.len() && key > &self.keys[i] {
            i += 1;
        }

        if i < self.keys.len() && key == &self.keys[i] {
            return true;
        }

        if self.is_leaf {
            return false;
        }

        self.children[i].search(key)
    }

    fn insert_non_full(&mut self, key: K) {
        let mut i = self.keys.len();

        if self.is_leaf {
            self.keys.push(key);
            i = self.keys.len() - 1;
            while i > 0 && self.keys[i] < self.keys[i - 1] {
                self.keys.swap(i, i - 1);
                i -= 1;
            }
        } else {
            while i > 0 && key < self.keys[i - 1] {
                i -= 1;
            }

            if self.children[i].is_full() {
                self.split_child(i);
                if key > self.keys[i] {
                    i += 1;
                }
            }
            self.children[i].insert_non_full(key);
        }
    }

    fn split_child(&mut self, i: usize) {
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
}

impl<K: Ord + std::fmt::Debug> BTree<K> {
    pub fn new() -> Self {
        BTree { root: None }
    }

    pub fn insert(&mut self, key: K) {
        match &mut self.root {
            None => {
                let mut root = Box::new(Node::new(true));
                root.keys.push(key);
                self.root = Some(root);
            }
            Some(root) => {
                if root.is_full() {
                    let mut new_root = Box::new(Node::new(false));
                    let old_root = self.root.take().unwrap();
                    new_root.children.push(old_root);
                    new_root.split_child(0);
                    new_root.insert_non_full(key);
                    self.root = Some(new_root);
                } else {
                    root.insert_non_full(key);
                }
            }
        }
    }

    pub fn search(&self, key: &K) -> bool {
        match &self.root {
            None => false,
            Some(root) => root.search(key),
        }
    }

    pub fn print_tree(&self) {
        if let Some(root) = &self.root {
            Self::print_node(root, 0);
        } else {
            println!("Empty tree");
        }
    }

    fn print_node(node: &Node<K>, level: usize)
    where
        K: std::fmt::Debug,
    {
        print!("{}", "  ".repeat(level));
        println!("Keys: {:?}", node.keys);

        if !node.is_leaf {
            for child in &node.children {
                Self::print_node(child, level + 1);
            }
        }
    }
}

fn main() {
    let mut btree = BTree::new();

    // Insert some values
    let values = vec![10, 20, 5, 6, 12, 30, 7, 17, 3, 16, 21, 22, 23, 24];

    println!("Inserting values: {:?}", values);
    for val in values {
        btree.insert(val);
    }

    println!("\nB-Tree structure:");
    btree.print_tree();

    // Search for some values
    println!("\nSearch results:");
    println!("Search for 6: {}", btree.search(&6));
    println!("Search for 15: {}", btree.search(&15));
    println!("Search for 21: {}", btree.search(&21));
}
