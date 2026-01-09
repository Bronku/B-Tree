# Persistent B+ Tree in Rust

A high-performance, disk-backed B+ Tree implementation designed for manual page management and efficient data retrieval. This project explores the intersection of database engine internals and Rust's memory safety.

## Key Features

* **Disk Persistence:** Implements a custom `FileStorage` layer that handles reading/writing nodes as discrete pages.
* **B+ Tree Logic:** Supports insertion, search, and tree traversal with internal/leaf node splitting.
* **Performance Tracking:** Built-in statistics for monitoring Page Reads, Page Writes, and total I/O operations.
* **REPL Interface:** Interactive command-line interface for real-time tree manipulation.

## Technical Implementation

The tree is built with a focus on **implementation-level transparency**:
- **Storage Abstraction:** Uses a `Storage` trait to allow for interchangeable backends (e.g., in-memory vs. file-backed).
- **Serialization:** Utilizes `bincode` for efficient binary encoding of nodes.
- **Node Structure:** Custom `Internal` and `Leaf` node definitions with configurable `DEGREE`.



## Getting Started

### Prerequisites
* Cargo

### Build and Run
```bash
cargo build --release
./target/release/btree my_database.db
```
## Repl Commands
Once running, you can use the following commands:
- `insert <k> <x> <a1> <a2> <a3> <a4> <a5>`- Insert a record (7 integers).
- `find <key>` - Search for a specific key.
- `tree` - Visual dump of the tree structure.
- `stats` - Show I/O performance (Reads/Writes).
- `exit` - Close the database.
