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

use std::fmt::{Debug, Formatter};

use crate::{
    debug::*,
    parser::{Error, ParseResult, SuperBox},
    BoxType,
};

/// Represents a single JUMBF box.
///
/// This is referred to here as a "data box" since it is intended to house
/// application-specific data. This crate does not ascribe any meaning to the
/// type field or the contents of this box.
///
/// A box is defined as a four-byte data type and a byte-slice payload
/// of any size. The contents of the payload will vary depending on the
/// data type.
#[derive(Clone, Eq, PartialEq)]
pub struct DataBox<'a> {
    /// Box type.
    ///
    /// This field specifies the type of information found in the `data`
    /// field. The value of this field is encoded as a 4-byte big-endian
    /// unsigned integer. However, boxes are generally referred to by an
    /// ISO/IEC 646 character string translation of the integer value.
    ///
    /// For that reason, this is represented here as a 4-byte slice.
    ///
    /// The box type can typically be matched with a byte string constant (i.e.
    /// `b"jumd"`).
    pub tbox: BoxType,

    /// Box contents.
    ///
    /// This field contains the actual information contained within this box.
    /// The format of the box contents depends on the box type and will be
    /// defined individually for each type.
    pub data: &'a [u8],

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: &'a [u8],
}

impl<'a> DataBox<'a> {
    /// Parse a JUMBF box, and return a tuple of the remainder of the input and
    /// the parsed box.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice(original: &'a [u8]) -> ParseResult<'a, Self> {
        // Read 4-byte length field.
        if original.len() < 4 {
            return Err(Error::Incomplete(4 - original.len()));
        }

        let len = u32::from_be_bytes([original[0], original[1], original[2], original[3]]);
        let i = &original[4..];

        // Read 4-byte box type.
        let (i, tbox): (&'a [u8], BoxType) = if i.len() >= 4 {
            let (tbox, i) = i.split_at(4);
            (i, tbox.into())
        } else {
            return Err(Error::Incomplete(4 - i.len()));
        };

        // Determine actual data length.
        let (i, len, original_len) = match len {
            0 => (i, i.len(), original.len()),

            1 => {
                // Extended length: read 8-byte length field.
                if i.len() < 8 {
                    return Err(Error::Incomplete(8 - i.len()));
                }

                let len = u64::from_be_bytes([i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7]]);
                let i = &i[8..];

                if len >= 16 {
                    (i, len as usize - 16, len as usize)
                } else {
                    return Err(Error::InvalidBoxLength(len as u32));
                }
            }

            2..=7 => {
                return Err(Error::InvalidBoxLength(len));
            }

            len => (i, len as usize - 8, len as usize),
        };

        // Extract data payload.
        if i.len() >= len {
            let (data, i) = i.split_at(len);
            Ok((
                i,
                Self {
                    tbox,
                    data,
                    original: &original[0..original_len],
                },
            ))
        } else {
            Err(Error::Incomplete(len - i.len()))
        }
    }

    /// Returns the offset of the *data* portion of this box within its
    /// enclosing [`SuperBox`].
    ///
    /// Will return `None` if this box is not a member of the [`SuperBox`].
    ///
    /// ## Example
    ///
    /// ```
    /// use hex_literal::hex;
    /// use jumbf::parser::SuperBox;
    ///
    /// let jumbf = hex!(
    ///     "00000077" // box size
    ///     "6a756d62" // box type = 'jumb'
    ///         "00000028" // box size
    ///         "6a756d64" // box type = 'jumd'
    ///         "6332637300110010800000aa00389b71" // UUID
    ///         "03" // toggles
    ///         "633270612e7369676e617475726500" // label
    ///         // ----
    ///         "00000047" // box size
    ///         "75756964" // box type = 'uuid'
    ///         "6332637300110010800000aa00389b717468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e" // data (type unknown)
    ///     );
    ///
    /// let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    /// assert!(rem.is_empty());
    ///
    /// let uuid_box = sbox.data_box().unwrap();
    /// assert_eq!(uuid_box.offset_within_superbox(&sbox), Some(56));
    /// ```
    pub fn offset_within_superbox(&self, super_box: &SuperBox) -> Option<usize> {
        let sbox_as_ptr = super_box.original.as_ptr() as usize;
        let self_as_ptr = self.data.as_ptr() as usize;

        if self_as_ptr < sbox_as_ptr {
            return None;
        }

        let offset = self_as_ptr.wrapping_sub(sbox_as_ptr);
        if offset + self.data.len() > super_box.original.len() {
            None
        } else {
            Some(offset)
        }
    }
}

impl<'a> Debug for DataBox<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DataBox")
            .field("tbox", &self.tbox)
            .field("data", &DebugByteSlice(self.data))
            .field("original", &DebugByteSlice(self.original))
            .finish()
    }
}
