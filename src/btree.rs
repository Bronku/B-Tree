use crate::node::Node;
use crate::record::Record;

#[derive(Debug)]
pub struct BTree {
    root: Option<Box<Node>>,
}

impl BTree {
    pub fn new() -> Self {
        BTree { root: None }
    }

    pub fn insert(&mut self, rec: Record) {
        match &mut self.root {
            None => {
                let mut root = Box::new(Node::new(true));
                root.keys.push(rec);
                self.root = Some(root);
            }
            Some(root) => {
                if root.is_full() {
                    let mut new_root = Box::new(Node::new(false));
                    let old_root = self.root.take().unwrap();
                    new_root.children.push(old_root);
                    new_root.split_child(0);
                    new_root.insert_non_full(rec);
                    self.root = Some(new_root);
                } else {
                    root.insert_non_full(rec);
                }
            }
        }
    }

    pub fn search(&self, key: i32) -> Option<Record> {
        match &self.root {
            None => None,
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

    fn print_node(node: &Node, level: usize) {
        print!("{}", "  ".repeat(level));

        let key_values: Vec<i32> = node.keys.iter().map(|r| r.key).collect();
        println!("Keys: {:?}", key_values);

        if !node.is_leaf {
            for child in &node.children {
                Self::print_node(child, level + 1);
            }
        }
    }
}
