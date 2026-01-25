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

#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![deny(warnings)]
#![doc = include_str!("../README.md")]

mod box_type;
pub use box_type::BoxType;

pub mod builder;

mod debug;

pub mod parser;

mod toggles;

#[cfg(test)]
mod tests {
    #[test]
    fn test_readme_example() {
        use std::{cell::RefCell, fs::File, io::BufReader, rc::Rc};

        use crate::parser::SuperBox;

        let file = File::open("src/tests/fixtures/C.c2pa").unwrap();
        let reader = Rc::new(RefCell::new(BufReader::new(file)));
        let sbox = SuperBox::from_reader(reader).unwrap();

        if let Some(child) = sbox.child_boxes.first() {
            let _super_box = child.as_super_box().unwrap();
            // ... dig deeper into nested boxes ...
        }
    }
}
