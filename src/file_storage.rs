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
