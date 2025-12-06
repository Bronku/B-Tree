mod btree;
mod node;
mod record;

use btree::BTree;
use record::Record;

fn main() {
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
}
