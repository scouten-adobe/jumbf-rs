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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use hex_literal::hex;
    use pretty_assertions_sorted::assert_eq;

    use crate::{
        parser::{DataBox, DescriptionBox, Error},
        BoxType,
    };

    #[test]
    fn from_slice() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (rem, dbox) = DescriptionBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.descbox",),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: None,\n    original: 38 bytes starting with [00, 00, 00, 26, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn from_box() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.descbox",),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: None,\n    original: 38 bytes starting with [00, 00, 00, 26, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn with_id() {
        let jumbf = hex!(
            "0000001d" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "04" // toggles
            "00001000" // ID
        );

        let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: None,
                requestable: false,
                id: Some(4096),
                hash: None,
                private: None,
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: None,\n    requestable: false,\n    id: Some(\n        4096,\n    ),\n    hash: None,\n    private: None,\n    original: 29 bytes starting with [00, 00, 00, 1d, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn error_incomplete_id() {
        let jumbf = hex!(
            "0000001c" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "04" // toggles
            "000010" // ID (incomplete)
        );

        assert_eq!(
            DescriptionBox::from_slice(&jumbf).unwrap_err(),
            Error::Incomplete(1)
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

        let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.descbox",),
                requestable: true,
                id: None,
                hash: Some(b"This is a bogus hash............" as &[u8; 32]),
                private: None,
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: Some(32 bytes starting with [54, 68, 69, 73, 20, 69, 73, 20, 61, 20, 62, 6f, 67, 75, 73, 20, 68, 61, 73, 68]),\n    private: None,\n    original: 70 bytes starting with [00, 00, 00, 46, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn with_private_box() {
        let jumbf = hex!(
                "0000004f" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "13" // toggles
                "746573742e64657363626f7800" // label
                    "00000029" // box size
                    "6a736f6e" // box type = 'json'
                    "7b20226c6f636174696f6e223a20224d61726761"
                    "746520436974792c204e4a227d" // payload (JSON)
        );

        let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.descbox",),
                requestable: true,
                id: None,
                hash: None,
                private: Some(DataBox {
                    tbox: BoxType(*b"json"),
                    data: &[
                        123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58, 32, 34, 77, 97,
                        114, 103, 97, 116, 101, 32, 67, 105, 116, 121, 44, 32, 78, 74, 34, 125,
                    ],
                    original: &jumbf[38..79],
                }),
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: Some(\n        DataBox {\n            tbox: b\"json\",\n            data: 33 bytes starting with [7b, 20, 22, 6c, 6f, 63, 61, 74, 69, 6f, 6e, 22, 3a, 20, 22, 4d, 61, 72, 67, 61],\n            original: 41 bytes starting with [00, 00, 00, 29, 6a, 73, 6f, 6e, 7b, 20, 22, 6c, 6f, 63, 61, 74, 69, 6f, 6e, 22],\n        },\n    ),\n    original: 79 bytes starting with [00, 00, 00, 4f, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn error_wrong_box_type() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d63" // box type = 'jumc' (INCORRECT)
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        assert_eq!(
            DescriptionBox::from_slice(&jumbf).unwrap_err(),
            Error::InvalidDescriptionBoxType(BoxType(*b"jumc"))
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

        let (rem, dbox) = DescriptionBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            dbox,
            DescriptionBox {
                uuid: &[0; 16],
                label: None,
                requestable: false,
                id: None,
                hash: None,
                private: None,
                original: &jumbf,
            }
        );

        assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: None,\n    requestable: false,\n    id: None,\n    hash: None,\n    private: None,\n    original: 25 bytes starting with [00, 00, 00, 19, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
    }

    #[test]
    fn error_incomplete_hash() {
        let jumbf = hex!(
            "00000044" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "0b" // toggles
            "746573742e64657363626f7800" // label
            "54686973206973206120626f67757320"
            "686173682e2e2e2e2e2e2e2e2e2e" // hash (incomplete)
        );

        assert_eq!(
            DescriptionBox::from_slice(&jumbf).unwrap_err(),
            Error::Incomplete(2)
        );
    }
}
