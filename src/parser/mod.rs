// Copyright 2024 Adobe. All rights reserved.
// This file is licensed to you under the Apache License,
// Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
// or the MIT license (http://opensource.org/licenses/MIT),
// at your option.

// Unless required by applicable law or agreed to in writing,
// this software is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR REPRESENTATIONS OF ANY KIND, either express or
// implied. See the LICENSE-MIT and LICENSE-APACHE files for the
// specific language governing permissions and limitations under
// each license.

//! An efficient parser for [JUMBF (ISO/IEC 19566-5:2019)] data structures.
//!
//! This module provides two complementary APIs for parsing JUMBF data:
//!
//! # Slice-based API (zero-copy)
//!
//! Use [`DataBox::from_slice`], [`DescriptionBox::from_slice`], or
//! [`SuperBox::from_slice`] when your JUMBF data is already in memory.
//! These methods perform zero-copy parsing for maximum performance.
//!
//! ```rust
//! use hex_literal::hex;
//! use jumbf::parser::{SuperBox, InputData, Label};
//!
//! let jumbf = hex!(
//!     "0000002f" // box size
//!     "6a756d62" // box type = 'jumb'
//!         "00000027" // box size
//!         "6a756d64" // box type = 'jumd'
//!         "00000000000000000000000000000000" // UUID
//!         "03" // toggles
//!         "746573742e7375706572626f7800" // label
//! );
//!
//! let (sbox, remaining) = SuperBox::from_slice(&jumbf).unwrap();
//! assert!(remaining.is_empty());
//! assert_eq!(sbox.desc.label, Some(Label::Borrowed("test.superbox")));
//! ```
//!
//! # Reader-based API (streaming)
//!
//! Use [`DataBox::from_reader`] or [`SuperBox::from_reader`] when parsing
//! from files or other I/O sources. This avoids loading the entire file into
//! memory and defers data copying until explicitly requested via
//! [`InputData::to_vec`].
//!
//! ```rust,no_run
//! use std::{cell::RefCell, fs::File, io::BufReader, rc::Rc};
//!
//! use jumbf::parser::SuperBox;
//!
//! let file = File::open("manifest.jumbf").unwrap();
//! let reader = Rc::new(RefCell::new(BufReader::new(file)));
//! let sbox = SuperBox::from_reader(reader).unwrap();
//!
//! // Data is only read from file when to_vec() is called.
//! if let Some(child) = sbox.child_boxes.first() {
//!     let data_box = child.as_data_box().unwrap();
//!     let bytes = data_box.data.to_vec().unwrap();
//! }
//! ```
//!
//! [JUMBF (ISO/IEC 19566-5:2019)]: https://www.iso.org/standard/73604.html

mod data_box;
mod description_box;
mod error;
mod input_data;
mod input_slice;
mod no_reader;
mod super_box;

pub use data_box::DataBox;
pub use description_box::{DescriptionBox, Label};
pub use error::Error;
pub use input_data::InputData;
pub use input_slice::InputSlice;
pub use no_reader::NoReader;
pub use super_box::{ChildBox, SuperBox};
