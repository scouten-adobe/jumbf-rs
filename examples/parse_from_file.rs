//! Example: Parse JUMBF from a file using streaming I/O.
//!
//! This demonstrates the reader-based API, which avoids loading the entire
//! file into memory. Data is only read when explicitly requested via to_vec().

use std::{cell::RefCell, fs::File, io::BufReader, rc::Rc};

use jumbf::parser::{ChildBox, SuperBox};

fn main() {
    // Path to a JUMBF file.
    let path = "src/tests/fixtures/C.c2pa";

    println!("Parsing JUMBF from file: {}", path);

    // Open the file and create a buffered reader.
    let file = File::open(path).expect("Failed to open file");
    let reader = Rc::new(RefCell::new(BufReader::new(file)));

    // Parse the JUMBF superbox from the reader.
    // At this point, only the box structure has been read,
    // not the actual payload data.
    let sbox = SuperBox::from_reader(reader).expect("Failed to parse JUMBF");

    println!("Parsed JUMBF SuperBox:");
    println!("  Label: {:?}", sbox.desc.label);
    println!("  Requestable: {}", sbox.desc.requestable);
    println!("  Child boxes: {}", sbox.child_boxes.len());

    // Access child boxes.
    for (i, child) in sbox.child_boxes.iter().enumerate() {
        match child {
            ChildBox::SuperBox(sb) => {
                println!("  Child {}: SuperBox with label {:?}", i, sb.desc.label);
                println!("    Nested children: {}", sb.child_boxes.len());
            }
            ChildBox::DataBox(db) => {
                println!("  Child {}: DataBox of type {:?}", i, db.tbox);
                println!("    Data length: {} bytes", db.data.len());

                // Data is only read from file when to_vec() is called.
                // For large files, you can choose to skip reading certain boxes.
                if db.data.len() < 1000 {
                    match db.data.to_vec() {
                        Ok(bytes) => {
                            println!("    Read {} bytes from file", bytes.len());
                        }
                        Err(e) => {
                            eprintln!("    Failed to read data: {}", e);
                        }
                    }
                } else {
                    println!("    (Skipping read of large data box)");
                }
            }
        }
    }
}
