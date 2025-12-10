use crate::record::Record;

#[derive(Debug, Clone)]
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

#[derive(Debug, Clone)]
pub struct LeafNode {
    pub keys: Vec<i32>,
    pub values: Vec<Record>,
    pub next: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct InternalNode {
    pub keys: Vec<i32>,
    pub children: Vec<usize>,
}
