# rethink-driver

A high performance RethinkDB driver written in Rust.

## Setup

Add this repository to your project's Cargo.toml:

```toml
rethink-driver = { git = "https://gitlab.com/TobiP64/rethink-driver.git" }
```

## Features

- tls (default): enables wire encryption via TLS, uses rustls
- async: enables the driver's async API

## Usage

```rust
use {rethink_driver::*, serde::Deserialize};

#[derive(Clone, Debug, Deserialize)]
struct TestDocument {
    id: String
}

fn main() {
    // connect to a server
    let conn = ConnectionOptions::default()
        .connect()
        .unwrap();

    // get all documents from table `test` in db `test`
    let cursor = db("test")
        .table("test")
        .run::<TestDocument>(&conn, None)
        .unwrap();
    
    for doc in cursor {
        println!("{:?}", doc);
    }
}
```