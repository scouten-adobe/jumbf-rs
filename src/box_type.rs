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

use std::fmt::{Debug, Error, Formatter};

/// A JUMBF "box type" is encoded as a 4-byte big-endian
/// unsigned integer. However, boxes are generally referred to by an
/// ISO/IEC 646 character string translation of the integer value.
///
/// For that reason, this is represented here as a 4-byte slice.
///
/// The box type can typically be matched with a byte string constant (i.e.
/// `b"jumd"`).
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct BoxType(pub [u8; 4]);

impl Debug for BoxType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.0.iter().all(|c| *c >= 0x20 && *c <= 0x7e) {
            write!(
                f,
                "b\"{}{}{}{}\"",
                self.0[0] as char, self.0[1] as char, self.0[2] as char, self.0[3] as char,
            )
        } else {
            write!(
                f,
                "[0x{:02x?}, 0x{:02x?}, 0x{:02x?}, 0x{:02x?}]",
                self.0[0], self.0[1], self.0[2], self.0[3]
            )
        }
    }
}

impl From<&[u8]> for BoxType {
    fn from(t: &[u8]) -> Self {
        let mut tbox = *b"    ";
        for (i, c) in t.iter().take(4).enumerate() {
            tbox[i] = *c;
        }
        Self(tbox)
    }
}

impl From<&[u8; 4]> for BoxType {
    fn from(t: &[u8; 4]) -> Self {
        Self(*t)
    }
}

/// Box type for JUMBF description box (`b"jumd"`).
pub const DESCRIPTION_BOX_TYPE: BoxType = BoxType(*b"jumd");

/// Box type for JUMBF super box (`b"jumb"`).
pub const SUPER_BOX_TYPE: BoxType = BoxType(*b"jumb");

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use crate::BoxType;

    #[test]
    fn impl_debug() {
        let x = BoxType([1, 2, 3, 4]);
        assert_eq!(format!("{x:#?}"), "[0x01, 0x02, 0x03, 0x04]");

        let x = BoxType(*b"abcd");
        assert_eq!(format!("{x:#?}"), "b\"abcd\"");

        let x = BoxType([b'a', b'b', b'c', 0x7f]);
        assert_eq!(format!("{x:#?}"), "[0x61, 0x62, 0x63, 0x7f]");
    }
}
