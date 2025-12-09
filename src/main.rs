mod btree;
mod consts;
mod node;
mod node_storage;
mod record;

use crate::node::Node;
use crate::node_storage::NodeStorage;
use crate::record::Record;

fn main() {
    let mut storage = NodeStorage::open("btree.idx");

    let mut node = storage.read_node(1);
    node.insert_non_full(Record {
        key: 1,
        a: [69, 0, 0, 0, 0],
        x: 0,
    });

    println!("{:?}", node);

    storage.write_node(1, &node);

    let node1 = storage.read_node(1);

    println!("{:?}", node1);

    println!("Record = {} bytes", std::mem::size_of::<Record>());
    println!("Node   = {} bytes", std::mem::size_of::<Node>());

    /*
    let mut btree = BTree::new();

    let mut values: Vec<i32> = vec![];

    println!("Inserting values");
    for _ in 0..10 {
        let rec = Record::random();
        values.push(rec.key);
        btree.insert(rec);
    }

    println!("\nB-Tree structure:");
    btree.print_tree();

    println!("\nSearch results:");
    println!("Search for 6: {:?}", btree.search(values[0]));
    println!("Search for 15: {:?}", btree.search(values[4]));
    println!("Search for 21: {:?}", btree.search(100));
    */
}
