use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use crate::node::Node;
use crate::record::Record;

pub const PAGE_SIZE: usize = 512;

#[derive(Debug)]
pub struct NodeStorage {
    file: File,
    pub page_reads: usize,
    pub page_writes: usize,
}

impl NodeStorage {
    pub fn open(path: &str) -> Self {
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
    fn serialize_node(node: &Node) -> [u8; PAGE_SIZE] {
        let mut out = String::new();

        // Format:
        // L|numkeys|key0;key1;...|child0,child1,...
        out.push(if node.is_leaf { 'L' } else { 'I' });
        out.push('|');
        out.push_str(&node.num_keys.to_string());
        out.push('|');

        // Keys
        for i in 0..node.num_keys {
            let rec = node.keys[i].unwrap();
            out.push_str(&rec.to_text());
            if i + 1 < node.num_keys {
                out.push(';');
            }
        }

        out.push('|');

        // Children (m = num_keys, m+1 children)
        for i in 0..=node.num_keys {
            match node.children[i] {
                Some(idx) => out.push_str(&idx.to_string()),
                None => out.push('.'),
            }

            if i < node.num_keys {
                out.push(',');
            }
        }

        out.push('|');

        // parent
        match node.parent {
            Some(p) => out.push_str(&p.to_string()),
            None => out.push('.'),
        }

        // Convert to fixed-size block
        let mut block = [b' '; PAGE_SIZE];
        let bytes = out.as_bytes();
        assert!(bytes.len() <= PAGE_SIZE, "Node too large to serialize");

        block[..bytes.len()].copy_from_slice(bytes);
        block
    }

    fn deserialize_node(block: &[u8; PAGE_SIZE]) -> Node {
        let text = std::str::from_utf8(block).unwrap().trim_end();
        let parts: Vec<&str> = text.split('|').collect();

        let mut node = Node::new(parts[0] == "L");
        node.num_keys = parts[1].parse().unwrap();

        if !parts[2].is_empty() {
            for (i, ks) in parts[2].split(';').enumerate() {
                let rec = Record::from_text(ks);
                node.keys[i] = Some(rec);
            }
        }

        if !parts[3].is_empty() {
            for (i, cs) in parts[3].split(',').enumerate() {
                node.children[i] = if cs == "." {
                    None
                } else {
                    Some(cs.parse().unwrap())
                }
            }
        }

        node.parent = if parts.len() > 4 && parts[4] != "." {
            Some(parts[4].parse().unwrap())
        } else {
            None
        };

        node
    }

    pub fn read_node(&mut self, index: usize) -> Node {
        let offset = (index * PAGE_SIZE) as u64;

        self.file.seek(SeekFrom::Start(offset)).unwrap();
        let mut block = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut block).unwrap();

        self.page_reads += 1;

        Self::deserialize_node(&block)
    }

    pub fn write_node(&mut self, index: usize, node: &Node) {
        let offset = (index * PAGE_SIZE) as u64;

        let block = Self::serialize_node(node);

        self.file.seek(SeekFrom::Start(offset)).unwrap();
        self.file.write_all(&block).unwrap();

        self.page_writes += 1;
    }

    pub fn append_node(&mut self, node: &Node) -> usize {
        let index = self.num_nodes();
        self.write_node(index, node);
        index
    }

    pub fn num_nodes(&self) -> usize {
        let len = self.file.metadata().unwrap().len() as usize;
        len / PAGE_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;
    use crate::record::Record;
    use rand::Rng;
    use tempfile::tempfile;

    fn random_node() -> Node {
        let mut node = Node::new(true);
        let mut rng = rand::rng();
        let n = rng.random_range(1..=crate::consts::MAX_KEYS);
        node.num_keys = n;
        for i in 0..n {
            node.keys[i] = Some(Record::random());
        }
        for i in 0..=n {
            node.children[i] = Some(rng.random_range(0..100));
        }

        node.parent = Some(rng.random_range(0..200));

        node
    }

    #[test]
    fn test_append_and_read_single_node() {
        let file = tempfile().unwrap();
        let mut storage = NodeStorage {
            file,
            page_reads: 0,
            page_writes: 0,
        };

        let node = random_node();
        let index = storage.append_node(&node);

        assert_eq!(index, 0);
        assert_eq!(storage.page_writes, 1);
        assert_eq!(storage.num_nodes(), 1);

        let read_node = storage.read_node(0);
        assert_eq!(read_node.num_keys, node.num_keys);

        for i in 0..node.num_keys {
            assert_eq!(read_node.keys[i].unwrap().key, node.keys[i].unwrap().key);
        }
        assert_eq!(storage.page_reads, 1);
        assert_eq!(read_node.parent, node.parent);
    }

    #[test]
    fn test_overwrite_node() {
        let file = tempfile().unwrap();
        let mut storage = NodeStorage {
            file,
            page_reads: 0,
            page_writes: 0,
        };

        let node1 = random_node();
        let node2 = random_node();

        let index = storage.append_node(&node1);
        storage.write_node(index, &node2);

        let read_node = storage.read_node(index);
        assert_eq!(read_node.num_keys, node2.num_keys);

        for i in 0..node2.num_keys {
            assert_eq!(read_node.keys[i].unwrap().key, node2.keys[i].unwrap().key);
        }

        assert_eq!(read_node.parent, node2.parent);

        assert_eq!(storage.page_writes, 2);
        assert_eq!(storage.page_reads, 1);
    }

    #[test]
    fn test_multiple_nodes() {
        let file = tempfile().unwrap();
        let mut storage = NodeStorage {
            file,
            page_reads: 0,
            page_writes: 0,
        };

        let mut nodes = vec![];

        // append 10 random nodes
        for _ in 0..10 {
            let node = random_node();
            storage.append_node(&node);
            nodes.push(node);
        }

        assert_eq!(storage.num_nodes(), 10);
        assert_eq!(storage.page_writes, 10);

        // read them back and verify
        for i in 0..10 {
            let read_node = storage.read_node(i);
            let orig = &nodes[i];

            assert_eq!(read_node.parent, orig.parent);
            assert_eq!(read_node.num_keys, orig.num_keys);
            for j in 0..orig.num_keys {
                assert_eq!(read_node.keys[j].unwrap().key, orig.keys[j].unwrap().key);
            }
        }

        assert_eq!(storage.page_reads, 10);
    }
}
