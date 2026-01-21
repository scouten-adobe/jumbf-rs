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
    fmt::{Debug, Formatter},
    str::from_utf8,
};

use crate::{
    box_type::DESCRIPTION_BOX_TYPE,
    debug::*,
    parser::{DataBox, Error, ParseResult},
};

/// A JUMBF description box describes the contents of its superbox.
///
/// This description contains a UUID and an optional text label, both
/// of which are specific to the application that is using JUMBF.
#[derive(Clone, Eq, PartialEq)]
pub struct DescriptionBox<'a> {
    /// Application-specific UUID for the superbox's data type.
    pub uuid: &'a [u8; 16],

    /// Application-specific label for the superbox.
    pub label: Option<&'a str>,

    /// True if the superbox containing this description box can
    /// be requested via [`SuperBox::find_by_label()`].
    ///
    /// [`SuperBox::find_by_label()`]: crate::parser::SuperBox::find_by_label
    pub requestable: bool,

    /// Application-specific 32-bit ID.
    pub id: Option<u32>,

    /// SHA-256 hash of the superbox's data payload.
    pub hash: Option<&'a [u8; 32]>,

    /// Application-specific "private" box within description box.
    pub private: Option<DataBox<'a>>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: &'a [u8],
}

impl<'a> DescriptionBox<'a> {
    /// Parse a JUMBF description box, and return a tuple of the remainder of
    /// the input and the parsed description box.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice(i: &'a [u8]) -> ParseResult<'a, Self> {
        let (i, boxx): (&'a [u8], DataBox<'a>) = DataBox::from_slice(i)?;
        let (_, desc) = Self::from_box(boxx)?;
        Ok((i, desc))
    }

    /// Convert an existing JUMBF box to a JUMBF description box.
    ///
    /// This consumes the existing [`DataBox`] object and will return an
    /// appropriate error if the box doesn't match the expected syntax for a
    /// description box.
    ///
    /// Returns a tuple of the remainder of the input from the box (which should
    /// typically be empty) and the new [`DescriptionBox`] object.
    pub fn from_box(boxx: DataBox<'a>) -> ParseResult<'a, Self> {
        use crate::toggles;

        if boxx.tbox != DESCRIPTION_BOX_TYPE {
            return Err(Error::InvalidDescriptionBoxType(boxx.tbox));
        }

        // Read 16-byte UUID.
        let (i, uuid): (&'a [u8], &'a [u8; 16]) = if boxx.data.len() >= 16 {
            let (uuid, i) = boxx.data.split_at(16);
            let uuid = uuid[0..16]
                .try_into()
                .map_err(|_| Error::Incomplete(16 - uuid.len()))?;
            (i, uuid)
        } else {
            return Err(Error::Incomplete(16 - boxx.data.len()));
        };

        // Read 1-byte toggles field.
        if i.is_empty() {
            return Err(Error::Incomplete(1));
        }
        let toggles = i[0];
        let i = &i[1..];

        // Toggle bit 0 (0x01) indicates that this superbox can be requested
        // via URI requests.
        let requestable = toggles & toggles::REQUESTABLE != 0;

        // Toggle bit 1 (0x02) indicates that the label has an optional textual label.
        let (i, label) = if toggles & toggles::HAS_LABEL != 0 {
            // Find null terminator.
            let null_pos = i.iter().position(|&b| b == 0).ok_or(Error::Incomplete(1))?;
            let label = from_utf8(&i[..null_pos]).map_err(Error::Utf8Error)?;
            (&i[null_pos + 1..], Some(label))
        } else {
            (i, None)
        };

        // Toggle bit 2 (0x04) indicates that the label has an optional
        // application-specific 32-bit identifier.
        let (i, id) = if toggles & toggles::HAS_ID != 0 {
            if i.len() < 4 {
                return Err(Error::Incomplete(4 - i.len()));
            }

            let id = u32::from_be_bytes([i[0], i[1], i[2], i[3]]);
            (&i[4..], Some(id))
        } else {
            (i, None)
        };

        // Toggle bit 3 (0x08) indicates that a SHA-256 hash of the superbox's
        // data box is present.
        let (i, hash) = if toggles & toggles::HAS_HASH != 0 {
            let (x, sig): (&'a [u8], &'a [u8; 32]) = if i.len() >= 32 {
                let (sig, x) = i.split_at(32);
                let sig = sig[0..32]
                    .try_into()
                    .map_err(|_| Error::Incomplete(32 - sig.len()))?;
                (x, sig)
            } else {
                return Err(Error::Incomplete(32 - i.len()));
            };

            (x, Some(sig))
        } else {
            (i, None)
        };

        // Toggle bit 4 (0x10) indicates that an application-specific "private"
        // box is contained within the description box.
        let (i, private) = if toggles & toggles::HAS_PRIVATE_BOX != 0 {
            let (i, private) = DataBox::from_slice(i)?;
            (i, Some(private))
        } else {
            (i, None)
        };

        Ok((
            i,
            Self {
                uuid,
                label,
                requestable,
                id,
                hash,
                private,
                original: boxx.original,
            },
        ))
    }
}

impl<'a> Debug for DescriptionBox<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DescriptionBox")
            .field("uuid", &DebugByteSlice(self.uuid))
            .field("label", &self.label)
            .field("requestable", &self.requestable)
            .field("id", &self.id)
            .field("hash", &DebugOption32ByteSlice(&self.hash))
            .field("private", &self.private)
            .field("original", &DebugByteSlice(self.original))
            .finish()
    }
}
