use crate::node::Node;

pub trait Storage {
    fn read_node(&mut self, loc: usize) -> Option<Node>;
    fn write_node(&mut self, loc: usize, node: &Node);
    fn total_nodes(&self) -> usize;
}
