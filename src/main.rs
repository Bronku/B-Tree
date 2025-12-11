mod btree;
mod config;
mod node;
mod record;
mod storage;
use crate::btree::BPlusTree;
use crate::storage::InMemoryStorage;

fn main() {
    let storage = InMemoryStorage::new();
    let mut tree = BPlusTree::open(storage);

    // Insert some key-value pairs
    tree.insert([1, 0, 0, 0, 0, 0, 0]);
    tree.insert([2, 1, 0, 1, 0, 1, 0]);

    // Find a key
    let value = tree.find(2);
    println!("Found value: {:?}", value);
}
