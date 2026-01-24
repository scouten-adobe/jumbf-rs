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

pub(crate) struct DebugByteSlice<'a>(pub(crate) &'a [u8]);

impl<'a> Debug for DebugByteSlice<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.0.len() > 20 {
            write!(
                f,
                "{} bytes starting with {:02x?}",
                self.0.len(),
                &self.0[0..20]
            )
        } else {
            write!(f, "{:02x?}", self.0)
        }
    }
}

pub(crate) struct DebugOption32ByteSlice<'a>(pub(crate) &'a Option<&'a [u8; 32]>);

impl<'a> Debug for DebugOption32ByteSlice<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if let Some(s) = self.0 {
            write!(f, "Some({:#?})", &DebugByteSlice(*s as &[u8]))
        } else {
            write!(f, "None")
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use hex_literal::hex;

    use crate::debug::*;

    #[test]
    fn debug_byte_slice() {
        let h = hex!("01020354595f");
        let s = DebugByteSlice(&h);
        assert_eq!(format!("{s:#?}"), "[01, 02, 03, 54, 59, 5f]");

        let h = hex!("000102030405060708090a0b0c0d0e0f10111213");
        let s = DebugByteSlice(&h);
        assert_eq!(
            format!("{s:#?}"),
            "[00, 01, 02, 03, 04, 05, 06, 07, 08, 09, 0a, 0b, 0c, 0d, 0e, 0f, 10, 11, 12, 13]"
        );

        let h = hex!("000102030405060708090a0b0c0d0e0f1011121314");
        let s = DebugByteSlice(&h);
        assert_eq!(
        format!("{s:#?}"),
        "21 bytes starting with [00, 01, 02, 03, 04, 05, 06, 07, 08, 09, 0a, 0b, 0c, 0d, 0e, 0f, 10, 11, 12, 13]"
    );
    }

    #[test]
    fn debug_option_32_byte_slice() {
        let h = hex!("54686973206973206120626f67757320686173682e2e2e2e2e2e2e2e2e2e2e2e");
        let h = Some(&h);

        let s = DebugOption32ByteSlice(&h);
        assert_eq!(format!("{s:#?}"), "Some(32 bytes starting with [54, 68, 69, 73, 20, 69, 73, 20, 61, 20, 62, 6f, 67, 75, 73, 20, 68, 61, 73, 68])");

        let s: DebugOption32ByteSlice = DebugOption32ByteSlice(&None);
        assert_eq!(format!("{s:#?}"), "None");
    }
}
