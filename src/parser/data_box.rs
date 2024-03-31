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
    parser::{Error, Source, SuperBox},
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
pub struct DataBox<S: Source> {
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
    pub data: S,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: S,
}

impl<S: Source> DataBox<S> {
    /// Parse a JUMBF box, and return a tuple of the remainder of the input and
    /// the parsed box.
    pub fn from_source(original: S) -> Result<(Self, S), Error<S::Error>> {
        let (len, i) = original.read_be32()?;

        let mut tbox = [0u8; 4];
        let i = i.read_bytes(&mut tbox)?;
        let tbox: BoxType = tbox.as_slice().into();

        let (len, i) = match len {
            0 => (i.len(), i),
            1 => {
                let (len, i) = i.read_be64()?;
                if len >= 16 {
                    (len as usize - 16, i)
                } else {
                    return Err(Error::InvalidBoxLength(len as u32));
                }
            }
            2..=7 => {
                return Err(Error::InvalidBoxLength(len));
            }
            len => (len as usize - 8, i),
        };

        let (data, i) = i.split_at(len)?;

        let (original, i) = original.split_at(original.len() - i.len())?;

        Ok((
            Self {
                tbox,
                data,
                original: original,
            },
            i,
        ))
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
    ///         "6332637300110010800000aa00389b717468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e"    // data (type unknown)
    ///     );
    ///
    /// let (sbox, rem) = SuperBox::from_source(jumbf.as_slice()).unwrap();
    /// assert!(rem.is_empty());
    ///
    /// let uuid_box = sbox.data_box().unwrap();
    /// assert_eq!(uuid_box.offset_within_superbox(&sbox), Some(56));
    /// ```
    pub fn offset_within_superbox(&self, super_box: &SuperBox<S>) -> Option<usize> {
        super_box.original.offset_of_subsource(&self.data)
    }
}

impl<S: Source + Debug> Debug for DataBox<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("DataBox")
            .field("tbox", &self.tbox)
            .field(
                "data",
                &DebugByteSlice(&self.data.as_bytes().unwrap_or_default()),
            )
            .field(
                "original",
                &DebugByteSlice(&self.original.as_bytes().unwrap_or_default()),
            )
            .finish()
    }
}
