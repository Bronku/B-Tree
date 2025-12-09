mod btree;
mod consts;
mod node;
mod node_storage;
mod record;

use crate::btree::BTree;
use crate::record::Record;

fn main() {
    let mut tree = BTree::new("btree.dat");
    tree.insert(Record {
        key: 1,
        a: [0, 0, 0, 0, 0],
        x: 0,
    });

    let rec = tree.search(0);
    println!("{:?}", rec);

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
