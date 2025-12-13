use bincode::error::DecodeError;

use crate::storage::Storage;
use crate::{config::PAGE_SIZE, node::Node};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug)]
pub struct FileStorage {
    pub file: File,
    pub page_reads: usize,
    pub page_writes: usize,
}

impl FileStorage {
    pub fn new(path: &str) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        Self {
            file,
            page_reads: 0,
            page_writes: 0,
        }
    }

    pub fn dump_pages(&mut self) {
        let total = self.total_nodes();
        for loc in 0..total {
            print!("Page {}: ", loc);

            match self.read_node(loc) {
                Some(Node::Header(header)) => {
                    println!("Header, root: {:?}", header.root)
                }
                Some(Node::Leaf(leaf)) => {
                    println!(
                        "Leaf keys={:?} values={} next={:?}",
                        leaf.keys,
                        leaf.values.len(),
                        leaf.next
                    );
                }
                Some(Node::Internal(internal)) => {
                    println!(
                        "Internal keys={:?} children={:?}",
                        internal.keys, internal.children
                    );
                }
                None => {
                    println!("<empty or invalid>");
                }
            }

            self.page_reads -= 1;
        }
    }
}

impl Storage for FileStorage {
    fn read_node(&mut self, loc: usize) -> Option<Node> {
        let offset = (loc * PAGE_SIZE) as u64;
        self.file.seek(SeekFrom::Start(offset)).ok()?;
        let mut block = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut block).ok()?;
        self.page_reads += 1;
        FileStorage::deserialize_node(block)
    }

    fn write_node(&mut self, loc: usize, input: &Node) {
        let offset = (loc * PAGE_SIZE) as u64;
        let block = FileStorage::serialize_node(input);
        self.file.seek(SeekFrom::Start(offset)).unwrap();
        self.file.write_all(&block).unwrap();
        self.page_writes += 1;
    }
    fn total_nodes(&self) -> usize {
        self.file.metadata().unwrap().len() as usize / PAGE_SIZE
    }
}

impl FileStorage {
    fn serialize_node(input: &Node) -> [u8; PAGE_SIZE] {
        let mut slice = [0u8; PAGE_SIZE];
        let _ = bincode::encode_into_slice(input, &mut slice, bincode::config::standard());
        return slice;
    }
    fn deserialize_node(input: [u8; PAGE_SIZE]) -> Option<Node> {
        let result: Result<(Node, usize), DecodeError> =
            bincode::decode_from_slice(&input, bincode::config::standard());
        match result {
            Ok(value) => Some(value.0),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::MAX_KEYS,
        node::{InternalNode, LeafNode, Node},
        record::Record,
    };
    use proptest::prelude::*;
    use tempfile::NamedTempFile;

    fn temp_storage() -> FileStorage {
        let file = NamedTempFile::new().unwrap();
        FileStorage::new(file.path().to_str().unwrap())
    }

    fn sample_leaf() -> Node {
        Node::Leaf(LeafNode {
            keys: vec![1, 2, 3],
            values: vec![[1; 7], [2; 7], [3; 7]],
            next: Some(42),
        })
    }

    fn sample_internal() -> Node {
        Node::Internal(InternalNode {
            keys: vec![10, 20],
            children: vec![1, 2, 3],
        })
    }

    #[test]
    fn write_and_read_leaf_node() {
        let mut storage = temp_storage();
        let node = sample_leaf();

        storage.write_node(0, &node);
        let read_back = storage.read_node(0);

        assert_eq!(read_back.unwrap(), node);
        assert_eq!(storage.page_reads, 1);
        assert_eq!(storage.page_writes, 1);
    }

    #[test]
    fn write_and_read_internal_node() {
        let mut storage = temp_storage();
        let node = sample_internal();

        storage.write_node(5, &node);
        let read_back = storage.read_node(5);

        assert_eq!(read_back.unwrap(), node);
    }

    #[test]
    fn read_empty_page_returns_none() {
        let mut storage = temp_storage();

        let result = storage.read_node(0);

        assert!(result.is_none());
        assert_eq!(storage.page_reads, 0);
    }

    #[test]
    fn total_nodes_is_correct() {
        let mut storage = temp_storage();

        storage.write_node(0, &sample_leaf());
        storage.write_node(1, &sample_internal());

        assert_eq!(storage.total_nodes(), 2);
    }

    #[test]
    fn overwrite_existing_node() {
        let mut storage = temp_storage();

        let node1 = sample_leaf();
        let node2 = sample_internal();

        storage.write_node(0, &node1);
        storage.write_node(0, &node2);

        let read_back = storage.read_node(0).unwrap();
        assert_eq!(read_back, node2);
    }

    fn arb_record() -> impl Strategy<Value = Record> {
        prop::array::uniform7(any::<i32>())
    }

    fn arb_node() -> impl Strategy<Value = Node> {
        let max = MAX_KEYS * 2 + 2;

        prop_oneof![
            (
                prop::collection::vec(any::<i32>(), 0..=max),
                prop::collection::vec(arb_record(), 0..=max),
                proptest::option::of(any::<usize>()),
            )
                .prop_map(|(keys, values, next)| { Node::Leaf(LeafNode { keys, values, next }) }),
            (
                prop::collection::vec(any::<i32>(), 0..=max),
                prop::collection::vec(any::<usize>(), 0..=max + 1),
            )
                .prop_map(|(keys, children)| { Node::Internal(InternalNode { keys, children }) })
        ]
    }

    fn arb_nodes() -> impl Strategy<Value = Vec<Node>> {
        prop::collection::vec(arb_node(), 0..100)
    }

    proptest! {
        #[test]
        fn node_roundtrip(node in arb_node()) {
            let mut storage = temp_storage();

            storage.write_node(0, &node);
            let read_back = storage.read_node(0).unwrap();

            prop_assert_eq!(node, read_back);
        }
    }

    proptest! {
        #[test]
        fn prop_multiple_nodes_roundtrip(nodes in arb_nodes()) {
            let mut storage = temp_storage();

            for (i, node) in nodes.iter().enumerate() {
                storage.write_node(i, node);
            }

            for (i, node) in nodes.iter().enumerate() {
                let read_back = storage.read_node(i).unwrap();
                prop_assert_eq!(read_back, node.clone());
            }
        }
    }

    proptest! {
        #[test]
        fn prop_overwrite_last_wins(nodes in arb_nodes()) {
            let mut storage = temp_storage();

            for node in &nodes {
                storage.write_node(0, node);
            }

            if let Some(last) = nodes.last() {
                let read_back = storage.read_node(0).unwrap();
                prop_assert_eq!(read_back, last.clone());
            }
        }
    }

    proptest! {
        #[test]
        fn prop_sparse_writes_are_isolated(
            a in arb_node(),
            b in arb_node(),
            loc_a in 0usize..20,
            loc_b in 0usize..20,
        ) {
            prop_assume!(loc_a != loc_b);

            let mut storage = temp_storage();

            storage.write_node(loc_a, &a);
            storage.write_node(loc_b, &b);

            let ra = storage.read_node(loc_a).unwrap();
            let rb = storage.read_node(loc_b).unwrap();

            prop_assert_eq!(ra, a);
            prop_assert_eq!(rb, b);
        }
    }

    proptest! {
        #[test]
        fn prop_total_nodes_matches_highest_write(
            nodes in arb_nodes()
        ) {
            let mut storage = temp_storage();

            for (i, node) in nodes.iter().enumerate() {
                storage.write_node(i * 2, node); // deliberately sparse
            }

            if !nodes.is_empty() {
                let expected_min = (nodes.len() - 1) * 2 + 1;
                prop_assert!(storage.total_nodes() >= expected_min);
            }
        }
    }

    proptest! {
        #[test]
        fn prop_reads_do_not_mutate_data(nodes in arb_nodes()) {
            let mut storage = temp_storage();

            for (i, node) in nodes.iter().enumerate() {
                storage.write_node(i, node);
            }

            for i in 0..nodes.len() {
                let _ = storage.read_node(i);
            }

            for (i, node) in nodes.iter().enumerate() {
                let read_back = storage.read_node(i).unwrap();
                prop_assert_eq!(read_back, node.clone());
            }
        }
    }

    proptest! {
        #[test]
        fn prop_unwritten_pages_return_none(
            nodes in arb_nodes(),
            extra in 1usize..20
        ) {
            let mut storage = temp_storage();

            for (i, node) in nodes.iter().enumerate() {
                storage.write_node(i, node);
            }

            let loc = nodes.len() + extra;
            let result = storage.read_node(loc);

            prop_assert!(result.is_none());
        }
    }
}
