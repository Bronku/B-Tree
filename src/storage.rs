use crate::node::Node;

pub trait Storage {
    fn read_node(&mut self, loc: usize) -> Option<Node>;
    fn write_node(&mut self, loc: usize, node: &Node);
    fn total_nodes(&self) -> usize;
}

// Example in-memory storage implementation for testing
pub struct InMemoryStorage {
    pub nodes: Vec<Option<Node>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

impl Storage for InMemoryStorage {
    fn read_node(&mut self, loc: usize) -> Option<Node> {
        self.nodes.get(loc)?.clone()
    }

    fn write_node(&mut self, loc: usize, node: &Node) {
        if loc >= self.nodes.len() {
            self.nodes.resize(loc + 1, None);
        }
        self.nodes[loc] = Some(node.clone());
    }

    fn total_nodes(&self) -> usize {
        self.nodes.len()
    }
}
