# jumbf

A [JUMBF (ISO/IEC 19566-5:2023)] parser and builder written in pure Rust.

[![CI](https://github.com/scouten-adobe/jumbf-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/scouten-adobe/jumbf-rs/actions/workflows/ci.yml) [![Latest Version](https://img.shields.io/crates/v/jumbf.svg)](https://crates.io/crates/jumbf) [![docs.rs](https://img.shields.io/docsrs/jumbf)](https://docs.rs/jumbf/latest/jumbf/) [![codecov](https://codecov.io/gh/scouten-adobe/jumbf-rs/graph/badge.svg?token=di7n9t9B80)](https://codecov.io/gh/scouten-adobe/jumbf-rs) [![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/scouten-adobe/jumbf-rs)

## Parser

This crate is intentionally minimal in its understanding of box content. Only `jumb` (superbox) and `jumd` (description box) content are understood. The content of all other box types (including other types described in the JUMBF standard) is generally application-specific and thus the meaning of that content is left to the caller.

The parser provides two APIs:

### Slice-based parsing (zero-copy)

When your JUMBF data is already in memory, use `from_slice()` for maximum performance with zero-copy parsing:

```rust
use hex_literal::hex;
use jumbf::parser::{DescriptionBox, InputData, Label, SuperBox};

let jumbf = hex!(
    "0000002f" // box size
    "6a756d62" // box type = 'jumb'
        "00000027" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "03" // toggles
        "746573742e7375706572626f7800" // label
);

let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
assert!(rem.is_empty());

assert_eq!(
    sbox,
    SuperBox {
        desc: DescriptionBox {
            uuid: [0; 16],
            label: Some(Label::Borrowed("test.superbox")),
            requestable: true,
            id: None,
            hash: None,
            private: None,
            original: InputData::Borrowed(&jumbf[8..47]),
        },
        child_boxes: vec!(),
        original: InputData::Borrowed(&jumbf),
    }
);
```

### Reader-based parsing (streaming)

When parsing from files or other I/O sources, use `from_reader()` to avoid loading the entire file into memory:

```rust,ignore
use std::{cell::RefCell, fs::File, io::BufReader, rc::Rc};

use jumbf::parser::SuperBox;

let file = File::open("src/tests/fixtures/C.c2pa").unwrap();
let reader = Rc::new(RefCell::new(BufReader::new(file)));
let sbox = SuperBox::from_reader(reader).unwrap();

if let Some(child) = sbox.child_boxes.first() {
    let _super_box = child.as_super_box().unwrap();
    // ... dig deeper into nested boxes ...
}
```

## Builder

This crate also allows you to build JUMBF data structures and serialize them.

```rust
use std::io::Cursor;

use hex_literal::hex;
use jumbf::{builder::{DataBoxBuilder, SuperBoxBuilder}, BoxType};

const JSON_BOX_TYPE: BoxType = BoxType(*b"json");
const RANDOM_BOX_TYPE: BoxType = BoxType(*b"abcd");

let child_box1 = DataBoxBuilder::from_owned(
    JSON_BOX_TYPE,
    hex!("7b20226c6f636174696f6e223a20224d61726761"
                "746520436974792c204e4a227d")
    .to_vec(),
);

let child_box2 = DataBoxBuilder::from_borrowed(RANDOM_BOX_TYPE, b"ABCD");

let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
    .add_child_box(child_box1)
    .add_child_box(child_box2);

let mut jumbf = Cursor::new(Vec::<u8>::new());
sbox.write_jumbf(&mut jumbf).unwrap();
```

## Contributions and feedback

We welcome contributions to this project. For information on contributing, providing feedback, and about ongoing work, see [Contributing](./CONTRIBUTING.md).

## Requirements

The crate requires **Rust version 1.88.0** or newer. When a newer version of Rust becomes required, a new minor (1.x.0) version of this crate will be released.

### Supported platforms

The crate has been tested on the following operating systems:

* Windows (IMPORTANT: Only the MSVC build chain is supported on Windows. We would welcome a PR to enable GNU build chain support on Windows.)
* MacOS (Intel and Apple silicon)
* Ubuntu Linux on x86 and ARM v8 (aarch64)

## License

The `jumbf` crate is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT).

Note that some components and dependent crates are licensed under different terms; please check the license terms for each crate and component for details.

## Changelog

Refer to the [CHANGELOG](https://github.com/scouten-adobe/jumbf-rs/blob/main/CHANGELOG.md) for detailed changes derived from Git commit history.

[JUMBF (ISO/IEC 19566-5:2023)]: https://www.iso.org/standard/84635.html
[thiserror]: https://crates.io/crates/thiserror
