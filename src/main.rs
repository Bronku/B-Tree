mod btree;
mod config;
mod file_storage;
mod node;
mod record;
mod storage;

use crate::{btree::BPlusTree, file_storage::FileStorage};
use std::{
    env,
    io::{self, Write},
};

fn main() {
    let filename = match env::args().nth(1) {
        Some(f) => f,
        None => {
            eprintln!("Usage: btree <database_file>");
            std::process::exit(1);
        }
    };

    let storage = FileStorage::new(&filename);
    let mut tree = BPlusTree::open(storage);

    repl(&mut tree);
}

fn repl(tree: &mut BPlusTree<FileStorage>) {
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read input");
            continue;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            break;
        }

        if input == "tree" {
            tree.dump_tree();
            continue;
        }

        if input == "all" {
            tree.dump_records();
            continue;
        }

        match handle_command(tree, input) {
            Ok(()) => {}
            Err(err) => println!("Error: {}", err),
        }
    }
}

fn handle_command(tree: &mut BPlusTree<FileStorage>, input: &str) -> Result<(), String> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    match parts.as_slice() {
        ["insert", rest @ ..] => handle_insert(tree, rest),
        ["find", key] => handle_find(tree, key),
        _ => Err("Unknown command".into()),
    }
}

fn handle_insert(tree: &mut BPlusTree<FileStorage>, args: &[&str]) -> Result<(), String> {
    if args.len() != 7 {
        return Err("Usage: insert k x a_1 a_2 a_3 a_4 a_5".into());
    }

    let mut record = [0i32; 7];
    for (i, arg) in args.iter().enumerate() {
        record[i] = arg
            .parse::<i32>()
            .map_err(|_| format!("Invalid integer: {}", arg))?;
    }

    tree.insert(record);
    println!("Inserted: {:?}", record);
    Ok(())
}

fn handle_find(tree: &mut BPlusTree<FileStorage>, key: &str) -> Result<(), String> {
    let key = key
        .parse::<i32>()
        .map_err(|_| format!("Invalid key: {}", key))?;

    match tree.find(key) {
        Some(record) => println!("Found: {:?}", record),
        None => println!("Key not found"),
    }

    Ok(())
}
