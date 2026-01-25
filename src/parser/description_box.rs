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
    fmt::{Debug, Formatter},
    io::{Read, Seek},
    rc::Rc,
    str::from_utf8,
};

use crate::{
    box_type::DESCRIPTION_BOX_TYPE,
    debug::*,
    parser::{DataBox, Error, InputData, NoReader},
};

/// A JUMBF description box describes the contents of its superbox.
///
/// This description contains a UUID and an optional text label, both
/// of which are specific to the application that is using JUMBF.
///
/// The generic parameter `R` represents the reader type for lazy-loaded data.
/// Small fields (UUID, label, hash) are always owned for reader-based parsing.
#[derive(Clone, Eq, PartialEq)]
pub struct DescriptionBox<'a, R = NoReader> {
    /// Application-specific UUID for the superbox's data type.
    /// Always owned (16 bytes is small enough to copy).
    pub uuid: [u8; 16],

    /// Application-specific label for the superbox.
    /// Borrowed for slice-based parsing, owned for reader-based parsing.
    pub label: Option<Label<'a>>,

    /// True if the superbox containing this description box can
    /// be requested via [`SuperBox::find_by_label()`].
    ///
    /// [`SuperBox::find_by_label()`]: crate::parser::SuperBox::find_by_label
    pub requestable: bool,

    /// Application-specific 32-bit ID.
    pub id: Option<u32>,

    /// SHA-256 hash of the superbox's data payload.
    /// Always owned (32 bytes is small enough to copy).
    pub hash: Option<[u8; 32]>,

    /// Application-specific "private" box within description box.
    pub private: Option<DataBox<'a, R>>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: InputData<'a, R>,
}

/// Label data that can be either borrowed or owned.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Label<'a> {
    /// Borrowed from input slice.
    Borrowed(&'a str),
    /// Owned string (from reader).
    Owned(String),
}

impl<'a> Label<'a> {
    /// Get the label as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(s) => s,
            Self::Owned(s) => s.as_str(),
        }
    }
}

impl<'a> DescriptionBox<'a, NoReader> {
    /// Parse a JUMBF description box, and return a tuple of the parsed
    /// description box and the remainder of the input.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice(i: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        let (boxx, i) = DataBox::from_slice(i)?;
        let (desc, _) = Self::from_box(boxx)?;
        Ok((desc, i))
    }

    /// Convert an existing JUMBF box to a JUMBF description box.
    ///
    /// This consumes the existing [`DataBox`] object and will return an
    /// appropriate error if the box doesn't match the expected syntax for a
    /// description box.
    ///
    /// Returns a tuple of the new [`DescriptionBox`] object and the remainder
    /// of the input from the box (which should typically be empty).
    pub fn from_box(boxx: DataBox<'a, NoReader>) -> Result<(Self, &'a [u8]), Error> {
        use crate::toggles;

        if boxx.tbox != DESCRIPTION_BOX_TYPE {
            return Err(Error::InvalidDescriptionBoxType(boxx.tbox));
        }

        // Get the data as a slice (must be borrowed for NoReader).
        let data = match boxx.data {
            InputData::Borrowed(slice) => slice,
            InputData::Lazy(_) => {
                return Err(Error::IoError("Expected borrowed data".to_string()));
            }
        };

        // Read 16-byte UUID.
        let (i, uuid): (&'a [u8], [u8; 16]) = if data.len() >= 16 {
            let (uuid_slice, i) = data.split_at(16);
            let uuid = uuid_slice[0..16]
                .try_into()
                .map_err(|_| Error::Incomplete(16 - uuid_slice.len()))?;
            (i, uuid)
        } else {
            return Err(Error::Incomplete(16 - data.len()));
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
            let label_str = from_utf8(&i[..null_pos]).map_err(Error::Utf8Error)?;
            (&i[null_pos + 1..], Some(Label::Borrowed(label_str)))
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
            let (x, hash_array): (&'a [u8], [u8; 32]) = if i.len() >= 32 {
                let (hash_slice, x) = i.split_at(32);
                let hash_array = hash_slice[0..32]
                    .try_into()
                    .map_err(|_| Error::Incomplete(32 - hash_slice.len()))?;
                (x, hash_array)
            } else {
                return Err(Error::Incomplete(32 - i.len()));
            };

            (x, Some(hash_array))
        } else {
            (i, None)
        };

        // Toggle bit 4 (0x10) indicates that an application-specific "private"
        // box is contained within the description box.
        let (i, private) = if toggles & toggles::HAS_PRIVATE_BOX != 0 {
            let (private, i) = DataBox::from_slice(i)?;
            (i, Some(private))
        } else {
            (i, None)
        };

        Ok((
            Self {
                uuid,
                label,
                requestable,
                id,
                hash,
                private,
                original: boxx.original,
            },
            i,
        ))
    }
}

impl<'a, R> Debug for DescriptionBox<'a, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DescriptionBox")
            .field("uuid", &DebugByteSlice(&self.uuid))
            .field("label", &self.label)
            .field("requestable", &self.requestable)
            .field("id", &self.id)
            .field("hash", &self.hash.as_ref().map(|h| DebugByteSlice(h)))
            .field("private", &self.private)
            .field("original", &self.original)
            .finish()
    }
}

impl<R: Read + Seek> DescriptionBox<'static, R> {
    /// Parse a JUMBF description box from a reader at its current position.
    ///
    /// The reader position will be advanced to the end of the description box
    /// upon success. Small fields (UUID, label, hash) are copied into owned
    /// data structures.
    pub fn from_reader(reader: Rc<RefCell<R>>) -> Result<Self, Error> {
        use crate::toggles;

        let data_box = DataBox::from_reader(Rc::clone(&reader))?;

        if data_box.tbox != DESCRIPTION_BOX_TYPE {
            return Err(Error::InvalidDescriptionBoxType(data_box.tbox));
        }

        // Read the data from the lazy InputData.
        let data = data_box.data.to_vec()?;

        // Parse UUID.
        if data.len() < 16 {
            return Err(Error::Incomplete(16 - data.len()));
        }
        let uuid: [u8; 16] = data[0..16].try_into().unwrap();
        let mut offset = 16;

        // Parse toggles.
        if offset >= data.len() {
            return Err(Error::Incomplete(1));
        }
        let toggles = data[offset];
        offset += 1;

        let requestable = toggles & toggles::REQUESTABLE != 0;

        // Parse label if present.
        let label = if toggles & toggles::HAS_LABEL != 0 {
            let null_pos = data[offset..]
                .iter()
                .position(|&b| b == 0)
                .ok_or(Error::Incomplete(1))?;
            let label_str =
                from_utf8(&data[offset..offset + null_pos]).map_err(Error::Utf8Error)?;
            offset += null_pos + 1;
            Some(Label::Owned(label_str.to_string()))
        } else {
            None
        };

        // Parse ID if present.
        let id = if toggles & toggles::HAS_ID != 0 {
            if offset + 4 > data.len() {
                return Err(Error::Incomplete(4 - (data.len() - offset)));
            }
            let id = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            Some(id)
        } else {
            None
        };

        // Parse hash if present.
        let hash = if toggles & toggles::HAS_HASH != 0 {
            if offset + 32 > data.len() {
                return Err(Error::Incomplete(32 - (data.len() - offset)));
            }
            let hash_array: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
            Some(hash_array)
        } else {
            None
        };

        // Parse private box if present.
        let private = if toggles & toggles::HAS_PRIVATE_BOX != 0 {
            // The private box is in the remaining data, but we need to parse it from the
            // reader.
            let private_box = DataBox::from_reader(Rc::clone(&reader))?;
            Some(private_box)
        } else {
            None
        };

        Ok(Self {
            uuid,
            label,
            requestable,
            id,
            hash,
            private,
            original: data_box.original,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use hex_literal::hex;
    use pretty_assertions_sorted::assert_eq;

    use crate::parser::{DataBox, DescriptionBox, Error, InputData, Label};

    #[test]
    fn from_slice() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (dbox, rem) = DescriptionBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: [0; 16],
                label: Some(Label::Borrowed("test.descbox")),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: InputData::Borrowed(&jumbf),
            }
        );
    }

    #[test]
    fn with_hash() {
        let jumbf = hex!(
            "00000046" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "0b" // toggles
            "746573742e64657363626f7800" // label
            "54686973206973206120626f67757320"
            "686173682e2e2e2e2e2e2e2e2e2e2e2e" // hash
        );

        let (boxx, rem) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        let (dbox, rem) = DescriptionBox::from_box(boxx).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: [0; 16],
                label: Some(Label::Borrowed("test.descbox")),
                requestable: true,
                id: None,
                hash: Some(*b"This is a bogus hash............"),
                private: None,
                original: InputData::Borrowed(&jumbf),
            }
        );
    }

    #[test]
    fn error_incomplete_uuid() {
        let jumbf = hex!(
            "00000016" // box size
            "6a756d64" // box type = 'jumd'
            "0000000000000000000000000000" // UUID (incomplete)
        );

        assert_eq!(
            DescriptionBox::from_slice(&jumbf).unwrap_err(),
            Error::Incomplete(2)
        );
    }

    #[test]
    fn no_label() {
        let jumbf = hex!(
            "00000019" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "00" // toggles
        );

        let (dbox, rem) = DescriptionBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: [0; 16],
                label: None,
                requestable: false,
                id: None,
                hash: None,
                private: None,
                original: InputData::Borrowed(&jumbf),
            }
        );
    }
}
