# jumbf

A [JUMBF (ISO/IEC 19566-5:2023)] parser and builder written in pure Rust.

[![CI](https://github.com/scouten-adobe/jumbf-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/scouten-adobe/jumbf-rs/actions/workflows/ci.yml) [![Latest Version](https://img.shields.io/crates/v/jumbf.svg)](https://crates.io/crates/jumbf) [![docs.rs](https://img.shields.io/docsrs/jumbf)](https://docs.rs/jumbf/latest/jumbf/) [![codecov](https://codecov.io/gh/scouten-adobe/jumbf-rs/graph/badge.svg?token=di7n9t9B80)](https://codecov.io/gh/scouten-adobe/jumbf-rs) [![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/scouten-adobe/jumbf-rs)

## Parser

The parser is implemented with the [nom] parser combinator framework and makes extensive use of zero-copy. Since the parsing features of this crate include dependencies on [nom] and [thiserror], those features are gated on a crate feature named `parser`, which is included by default.

This crate is intentionally minimal in its understanding of box content. Only `jumb` (superbox) and `jumd` (description box) content are understood. The content of all other box types (including other types described in the JUMBF standard) is generally application-specific and thus the meaning of that content is left to the caller.


```rust
use hex_literal::hex;
use jumbf::parser::{DescriptionBox, SuperBox};

let jumbf = hex!(
    "0000002f" // box size
    "6a756d62" // box type = 'jumb'
        "00000027" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "03" // toggles
        "746573742e7375706572626f7800" // label
);

let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
assert!(rem.is_empty());

assert_eq!(
    sbox,
    SuperBox {
        desc: DescriptionBox {
            uuid: &[0; 16],
            label: Some("test.superbox"),
            requestable: true,
            id: None,
            hash: None,
            private: None,
            original: &jumbf[8..47],
        },
        child_boxes: vec!(),
        original: &jumbf,
    }
);
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

### Reduced dependencies for builder only

The builder can be built by itself and has no third-party crate dependencies in that configuration. If you only need to _build_ JUMBF data structures and want to reduce compile-time overhead, you can disable the `parser` feature by importing this crate as follows:

```toml
jumbf = { version = "x.x", default-features = false }
```

## Contributions and feedback

We welcome contributions to this project. For information on contributing, providing feedback, and about ongoing work, see [Contributing](./CONTRIBUTING.md).

## Requirements

The crate requires **Rust version 1.80.0** or newer. When a newer version of Rust becomes required, a new minor (1.x.0) version of this crate will be released.

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
[nom]: https://github.com/rust-bakery/nom
[thiserror]: https://crates.io/crates/thiserror
