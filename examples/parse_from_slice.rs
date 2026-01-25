//! Example: Parse JUMBF from an in-memory byte slice.
//!
//! This demonstrates the zero-copy slice-based API, which is ideal when
//! your JUMBF data is already in memory.

use hex_literal::hex;
use jumbf::parser::{ChildBox, SuperBox};

fn main() {
    // Example JUMBF data with nested boxes.
    let jumbf = hex!(
        "00000077" // box size
        "6a756d62" // box type = 'jumb'
            "00000028" // box size
            "6a756d64" // box type = 'jumd'
            "6332637300110010800000aa00389b71" // UUID
            "03" // toggles
            "633270612e7369676e617475726500" // label = "c2pa.signature"
            // ----
            "00000047" // box size
            "75756964" // box type = 'uuid'
            "6332637300110010800000aa00389b71" // UUID
            "7468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e" // data
    );

    // Parse the JUMBF superbox from the slice.
    let (sbox, remaining) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(remaining.is_empty());

    println!("Parsed JUMBF SuperBox:");
    println!("  Label: {:?}", sbox.desc.label);
    println!("  Requestable: {}", sbox.desc.requestable);
    println!("  Child boxes: {}", sbox.child_boxes.len());

    // Access child boxes.
    for (i, child) in sbox.child_boxes.iter().enumerate() {
        match child {
            ChildBox::SuperBox(sb) => {
                println!("  Child {}: SuperBox with label {:?}", i, sb.desc.label);
            }
            ChildBox::DataBox(db) => {
                println!("  Child {}: DataBox of type {:?}", i, db.tbox);
                // For slice-based parsing, data is immediately available.
                if let Some(data_slice) = db.data.as_slice() {
                    println!("    Data length: {} bytes", data_slice.len());
                }
            }
        }
    }
}
