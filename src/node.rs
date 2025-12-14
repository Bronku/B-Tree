use bincode::{Decode, Encode};

use crate::config::DEGREE;
use crate::record::Record;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
    Header(HeaderNode),
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct HeaderNode {
    pub root: usize,
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct LeafNode {
    pub keys: Vec<i32>,
    pub values: Vec<Record>,
    pub next: Option<usize>,
}

impl LeafNode {
    pub fn new() -> Self {
        Self {
            keys: Vec::with_capacity(DEGREE * 2),
            values: Vec::with_capacity(DEGREE * 2),
            next: None,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct InternalNode {
    pub keys: Vec<i32>,
    pub children: Vec<usize>,
}
