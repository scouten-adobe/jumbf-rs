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

use std::{
    cell::RefCell,
    io::{Read, Seek},
};

pub enum CowSource<'a, T: Read + Seek> {
    Stream {
        source: RefCell<T>,
        start: usize,
        len: usize,
    },
    Owned(Vec<u8>),
    Borrowed(&'a [u8]),
}