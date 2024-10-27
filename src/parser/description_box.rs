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
    parser::{DataBox, Error, Source},
};

/// A JUMBF description box describes the contents of its superbox.
///
/// This description contains a UUID and an optional text label, both
/// of which are specific to the application that is using JUMBF.
#[derive(Clone, Eq, PartialEq)]
pub struct DescriptionBox<S: Source> {
    /// Application-specific UUID for the superbox's data type.
    pub uuid: [u8; 16],

    /// Application-specific label for the superbox.
    pub label: Option<String>,

    /// True if the superbox containing this description box can
    /// be requested via [`SuperBox::find_by_label()`].
    ///
    /// [`SuperBox::find_by_label()`]: crate::parser::SuperBox::find_by_label
    pub requestable: bool,

    /// Application-specific 32-bit ID.
    pub id: Option<u32>,

    /// SHA-256 hash of the superbox's data payload.
    pub hash: Option<[u8; 32]>,

    /// Application-specific "private" box within description box.
    pub private: Option<DataBox<S>>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: S,
}

impl<S: Source> DescriptionBox<S> {
    /// Parse a JUMBF description box, and return a tuple of the remainder of
    /// the input and the parsed description box.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_source(i: S) -> Result<(Self, S), Error<S::Error>> {
        let (dbox, rem) = DataBox::from_source(i)?;
        Ok((Self::from_data_box(dbox)?, rem))
    }

    /// Convert an existing JUMBF box to a JUMBF description box.
    ///
    /// This consumes the existing [`DataBox`] object and will return an
    /// appropriate error if the box doesn't match the expected syntax for a
    /// description box.
    ///
    /// Returns a tuple of the remainder of the input from the box (which should
    /// typically be empty) and the new [`DescriptionBox`] object.
    pub fn from_data_box(dbox: DataBox<S>) -> Result<Self, Error<S::Error>> {
        use crate::toggles;

        if dbox.tbox != DESCRIPTION_BOX_TYPE {
            return Err(Error::InvalidDescriptionBoxType(dbox.tbox));
        }

        let mut uuid = [0u8; 16];
        let i = dbox.data.read_bytes(&mut uuid)?;

        let mut toggles = [0u8];
        let i = i.read_bytes(&mut toggles)?;
        let toggles = toggles[0];

        // Toggle bit 0 (0x01) indicates that this superbox can be requested
        // via URI requests.
        let requestable = toggles & toggles::REQUESTABLE != 0;

        // Toggle bit 1 (0x02) indicates that the label has an optional textual label.
        let (label, i) = if toggles & toggles::HAS_LABEL != 0 {
            let (label, i) = i.split_at_null()?;

            let mut label_utf8 = vec![0u8; label.len()];
            label.read_bytes(&mut label_utf8)?;

            let label = from_utf8(&label_utf8).map_err(Error::Utf8Error)?;
            (Some(label.to_owned()), i)
        } else {
            (None, i)
        };

        // Toggle bit 2 (0x04) indicates that the label has an optional
        // application-specific 32-bit identifier.
        let (id, i) = if toggles & toggles::HAS_ID != 0 {
            let (id, i) = i.read_be32()?;
            (Some(id), i)
        } else {
            (None, i)
        };

        // Toggle bit 3 (0x08) indicates that a SHA-256 hash of the superbox's
        // data box is present.
        let (hash, i) = if toggles & toggles::HAS_HASH != 0 {
            let mut hash = [0u8; 32];
            let i = i.read_bytes(&mut hash)?;

            (Some(hash), i)
        } else {
            (None, i)
        };

        // Toggle bit 4 (0x10) indicates that an application-specific "private"
        // box is contained within the description box.
        let (private, _i) = if toggles & toggles::HAS_PRIVATE_BOX != 0 {
            let (private, i) = DataBox::from_source(i)?;
            (Some(private), i)
        } else {
            (None, i)
        };

        Ok(Self {
            uuid,
            label,
            requestable,
            id,
            hash,
            private,
            original: dbox.original,
        })
    }
}

impl<S: Source + Debug> Debug for DescriptionBox<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DescriptionBox")
            .field("uuid", &DebugByteSlice(self.uuid.as_slice()))
            .field("label", &self.label)
            .field("requestable", &self.requestable)
            .field("id", &self.id)
            .field("hash", &DebugOption32ByteSlice(&self.hash))
            .field("private", &self.private)
            .field(
                "original",
                &DebugByteSlice(&self.original.as_bytes().unwrap_or_default()),
            )
            .finish()
    }
}
