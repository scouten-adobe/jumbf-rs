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
    io::{Cursor, Read, Seek, SeekFrom},
    rc::Rc,
};

use crate::{
    parser::{Error, InputData, NoReader, SuperBox},
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
///
/// The generic parameter `R` represents the reader type for lazy-loaded data.
/// Use `DataBox<'a>` (or `DataBox<'a, NoReader>`) for slice-based parsing
/// (zero-copy) or `DataBox<'static, R>` where `R: Read + Seek` for reader-based
/// parsing.
#[derive(Clone, Eq, PartialEq)]
pub struct DataBox<'a, R = NoReader> {
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
    ///
    /// For slice-based parsing, this is a borrowed reference.
    /// For reader-based parsing, this is a lazy reference that reads on demand.
    pub data: InputData<'a, R>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    ///
    /// For slice-based parsing, this is a borrowed reference.
    /// For reader-based parsing, this is a lazy reference that reads on demand.
    pub original: InputData<'a, R>,
}

impl<'a, R> DataBox<'a, R> {
    /// Internal parsing method that works with any Read + Seek source.
    fn from_source<S: Read + Seek>(
        source: &mut S,
        create_input_data: impl Fn(u64, usize) -> InputData<'a, R>,
    ) -> Result<Self, Error> {
        let start_offset = source.stream_position()?;

        // Read 4-byte length field.
        let mut len_buf = [0u8; 4];
        source.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf);

        // Read 4-byte box type.
        let mut tbox_buf = [0u8; 4];
        source.read_exact(&mut tbox_buf)?;
        let tbox = BoxType(tbox_buf);

        // Determine actual data length.
        let (data_offset, data_len, original_len) = match len {
            0 => {
                // Read to end of stream.
                let current = source.stream_position()?;
                let end = source.seek(SeekFrom::End(0))?;
                source.seek(SeekFrom::Start(current))?;
                (
                    current,
                    (end - current) as usize,
                    (end - start_offset) as usize,
                )
            }
            1 => {
                // Extended length: read 8-byte length field.
                let mut xl_buf = [0u8; 8];
                source.read_exact(&mut xl_buf)?;
                let xl = u64::from_be_bytes(xl_buf);

                if xl < 16 {
                    return Err(Error::InvalidBoxLength(xl as u32));
                }

                let current = source.stream_position()?;
                (current, (xl - 16) as usize, xl as usize)
            }
            2..=7 => {
                return Err(Error::InvalidBoxLength(len));
            }
            len => {
                let current = source.stream_position()?;
                (current, (len as usize - 8), len as usize)
            }
        };

        // Advance to end of box.
        source.seek(SeekFrom::Start(start_offset + original_len as u64))?;

        Ok(Self {
            tbox,
            data: create_input_data(data_offset, data_len),
            original: create_input_data(start_offset, original_len),
        })
    }
}

impl<'a> DataBox<'a, NoReader> {
    /// Parse a JUMBF box, and return a tuple of the parsed box and
    /// the remainder of the input.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice(original: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        let mut cursor = Cursor::new(original);
        let data_box = Self::from_source(&mut cursor, |offset, len| {
            let start = offset as usize;
            let end = start + len;
            // Bounds are validated after from_source returns.
            // If out of bounds, we'll detect it when checking cursor position.
            if end <= original.len() {
                InputData::Borrowed(&original[start..end])
            } else {
                // Return empty slice - error will be caught below.
                InputData::Borrowed(&[])
            }
        })?;

        let pos = cursor.position() as usize;

        // Check if the box claimed more data than available.
        if pos > original.len() {
            return Err(Error::IoError(
                "Box size exceeds available data".to_string(),
            ));
        }

        Ok((data_box, &original[pos..]))
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
    /// let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
    /// assert!(rem.is_empty());
    ///
    /// let uuid_box = sbox.data_box().unwrap();
    /// assert_eq!(uuid_box.offset_within_superbox(&sbox), Some(56));
    /// ```
    pub fn offset_within_superbox(&self, super_box: &SuperBox) -> Option<usize> {
        // Both must be borrowed data for pointer comparison to work.
        let sbox_slice = super_box.original.as_slice()?;
        let data_slice = self.data.as_slice()?;

        let sbox_as_ptr = sbox_slice.as_ptr() as usize;
        let self_as_ptr = data_slice.as_ptr() as usize;

        if self_as_ptr < sbox_as_ptr {
            return None;
        }

        let offset = self_as_ptr.wrapping_sub(sbox_as_ptr);
        if offset + data_slice.len() > sbox_slice.len() {
            None
        } else {
            Some(offset)
        }
    }
}

impl<'a, R> Debug for DataBox<'a, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DataBox")
            .field("tbox", &self.tbox)
            .field("data", &self.data)
            .field("original", &self.original)
            .finish()
    }
}

impl<R: Read + Seek> DataBox<'static, R> {
    /// Parse a JUMBF box from a reader at its current position.
    ///
    /// The reader position will be advanced to the end of the box upon success.
    /// Data is stored as lazy references and only read when `.to_vec()` is
    /// called.
    pub fn from_reader(reader: Rc<RefCell<R>>) -> Result<Self, Error> {
        use crate::parser::InputSlice;

        Self::from_source(&mut *reader.borrow_mut(), |offset, len| {
            InputData::Lazy(InputSlice::new(Rc::clone(&reader), offset, len))
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

    use crate::{
        box_type::DESCRIPTION_BOX_TYPE,
        parser::{DataBox, Error, InputData},
    };

    #[test]
    fn simple_box() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (boxx, rem) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            boxx,
            DataBox {
                tbox: DESCRIPTION_BOX_TYPE,
                data: InputData::Borrowed(&[
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 116, 101, 115, 116, 46, 100,
                    101, 115, 99, 98, 111, 120, 0,
                ]),
                original: InputData::Borrowed(&jumbf),
            }
        );

        // Verify we can access the data.
        assert_eq!(boxx.data.as_slice().unwrap().len(), 30);
        assert_eq!(boxx.original.as_slice().unwrap(), &jumbf);
    }

    #[test]
    fn error_incomplete_box_length() {
        let jumbf = hex!(
        "000002" // box size (invalid, needs to be 32 bits)
    );

        // When using Cursor internally, incomplete data returns IoError
        assert!(matches!(
            DataBox::from_slice(&jumbf).unwrap_err(),
            Error::IoError(_)
        ));
    }

    #[test]
    fn error_incomplete_box_type() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d" // box type = 'jum' (missing last byte)
        );

        // When using Cursor internally, incomplete data returns IoError
        assert!(matches!(
            DataBox::from_slice(&jumbf).unwrap_err(),
            Error::IoError(_)
        ));
    }

    #[test]
    fn error_invalid_box_length() {
        let jumbf = hex!(
            "00000002" // box size (invalid)
            "6A756D62" // box type = 'jumb'
        );

        assert_eq!(
            DataBox::from_slice(&jumbf).unwrap_err(),
            Error::InvalidBoxLength(2)
        );
    }

    #[test]
    fn read_to_eof() {
        let jumbf = hex!(
            "00000000" // box size (read to EOF)
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (_boxx, rem) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());
    }

    #[test]
    fn read_xlbox_size() {
        let jumbf = hex!(
            "00000001" // box size (contained in xlbox)
            "6a756d64" // box type = 'jumd'
            "000000000000002e" // XLbox (extra long box size)
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let (boxx, rem) = DataBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        // Verify the parsed data is correct.
        assert_eq!(boxx.tbox, DESCRIPTION_BOX_TYPE);
        assert_eq!(boxx.data.len(), 30);
        assert_eq!(boxx.original.len(), jumbf.len());
    }

    #[test]
    fn error_xlbox_size_too_small() {
        let jumbf = hex!(
            "00000001" // box size (contained in xlbox)
            "6a756d64" // box type = 'jumd'
            "000000000000000e" // XLbox (INCORRECT extra long box size)
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        assert_eq!(
            DataBox::from_slice(&jumbf).unwrap_err(),
            Error::InvalidBoxLength(14)
        );
    }

    #[test]
    fn error_incorrect_length() {
        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            // label (missing)
        );

        // When using Cursor internally, incomplete data returns IoError
        assert!(matches!(
            DataBox::from_slice(&jumbf).unwrap_err(),
            Error::IoError(_)
        ));
    }

    #[test]
    fn from_reader() {
        use std::{cell::RefCell, io::Cursor, rc::Rc};

        let jumbf = hex!(
            "00000026" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e64657363626f7800" // label
        );

        let reader = Rc::new(RefCell::new(Cursor::new(jumbf.to_vec())));
        let data_box = DataBox::from_reader(reader).unwrap();

        assert_eq!(data_box.tbox, DESCRIPTION_BOX_TYPE);
        assert_eq!(data_box.data.len(), 30);
        assert_eq!(data_box.original.len(), 38);
        assert!(data_box.data.is_lazy());
        assert!(data_box.original.is_lazy());

        // Verify we can read the data on demand.
        let data_vec = data_box.data.to_vec().unwrap();
        assert_eq!(data_vec.len(), 30);
        assert_eq!(data_vec[0..16], [0; 16]); // UUID
        assert_eq!(data_vec[16], 3); // toggles
    }

    // Temporarily disabled until SuperBox is updated.
    mod offset_within_superbox {
        // The "happy path" cases for offset_within_superbox are
        // covered in the SuperBox test suite. This test suite is
        // intended to prove safe behavior given incorrect and/or
        // hostile inputs.

        use hex_literal::hex;
        use pretty_assertions_sorted::assert_eq;

        use crate::parser::SuperBox;

        #[test]
        #[ignore] // Disabled until SuperBox is updated
        fn abuse_read_to_eof() {
            // In this test case, we abuse JUMBF's ability to use 0
            // as the "box size" to mean read to "end of input."

            // We parse the same JUMBF superblock twice with different input
            // lengths, which means the pointers will align, but the data box
            // from the longer parse run will overrun the container of the
            // shorter parse run.

            // The `offset_within_superbox` code should detect this and
            // return `None` in this case.

            let jumbf = hex!(
            "00000000" // box size
            "6a756d62" // box type = 'jumb'
                "00000028" // box size
                "6a756d64" // box type = 'jumd'
                "6332637300110010800000aa00389b71" // UUID
                "03" // toggles
                "633270612e7369676e617475726500" // label
                // ----
                "00000000" // box size
                "75756964" // box type = 'uuid'
                "6332637300110010800000aa00389b717468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e" // data (type unknown)
            );

            let (sbox_full, rem) = SuperBox::from_slice(&jumbf).unwrap();
            assert!(rem.is_empty());

            assert_eq!(sbox_full.original.len(), 119);

            let (sbox_short, rem) = SuperBox::from_slice(&jumbf[0..118]).unwrap();

            assert!(rem.is_empty());
            assert_eq!(sbox_short.original.len(), 118);

            let dbox_from_full = sbox_full.data_box().unwrap();

            assert_eq!(
                dbox_from_full.offset_within_superbox(&sbox_full).unwrap(),
                56
            );
            assert!(dbox_from_full.offset_within_superbox(&sbox_short).is_none());

            let dbox_as_child = sbox_full.child_boxes.first().unwrap();
            assert!(dbox_as_child.as_super_box().is_none());

            let dbox_as_child = dbox_as_child.as_data_box().unwrap();
            assert_eq!(dbox_from_full, dbox_as_child);
        }

        #[test]
        #[ignore] // Disabled until SuperBox is updated
        fn dbox_precedes_sbox() {
            let jumbf = hex!(
                "00000267" // box size
                "6a756d62" // box type = 'jumb'
                    "0000001e" // box size
                    "6a756d64" // box type = 'jumd'
                    "6332706100110010800000aa00389b71" // UUID
                    "03" // toggles
                    "6332706100" // label = "c2pa"
                    // ---
                    "00000241" // box size
                    "6a756d62" // box type = 'jumb'
                        "00000024" // box size
                        "6a756d64" // box type = 'jumd'
                        "63326d6100110010800000aa00389b71" // UUID
                        "03" // toggles
                        "63622e61646f62655f3100" // label = "cb.adobe_1"
                        // ---
                        "0000008f" // box size
                        "6a756d62" // box type = 'jumb'
                            "00000029" // box size
                            "6a756d64" // box type = 'jumd'
                            "6332617300110010800000aa00389b71" // UUID
                            "03" // toggles
                            "633270612e617373657274696f6e7300" // label = "c2pa.assertions"
                            // ---
                            "0000005e" // box size
                            "6a756d62" // box type = 'jumb'
                                "0000002d" // box size
                                "6a756d64" // box type = 'jumd'
                                "6a736f6e00110010800000aa00389b71" // UUID
                                "03" // toggles
                                "633270612e6c6f636174696f6e2e62726f616400"
                                    // label = "c2pa.location.broad"
                                // ---
                                "00000029" // box size
                                "6a736f6e" // box type = 'json'
                                "7b20226c6f636174696f6e223a20224d61726761"
                                "746520436974792c204e4a227d" // payload (JSON)
                        // ---
                        "0000010f" // box size
                        "6a756d62" // box type = 'jumb'
                            "00000024" // box size
                            "6a756d64" // box type = 'jumd'
                            "6332636c00110010800000aa00389b71" // UUID
                            "03" // toggles
                            "633270612e636c61696d00" // label = "c2pa.claim"
                            // ---
                            "000000e3" // box size
                            "6a736f6e" // box type = 'json'
                            "7b0a2020202020202020202020202272"
                            "65636f7264657222203a202250686f74"
                            "6f73686f70222c0a2020202020202020"
                            "20202020227369676e61747572652220"
                            "3a202273656c66236a756d62663d735f"
                            "61646f62655f31222c0a202020202020"
                            "20202020202022617373657274696f6e"
                            "7322203a205b0a202020202020202020"
                            "202020202020202273656c66236a756d"
                            "62663d61735f61646f62655f312f6332"
                            "70612e6c6f636174696f6e2e62726f61"
                            "643f686c3d3736313432424436323336"
                            "3346220a202020202020202020202020"
                            "5d0a20202020202020207d" // payload (JSON)
                        // ---
                        "00000077" // box size
                        "6a756d62" // box type = 'jumb'
                            "00000028" // box size
                            "6a756d64" // box type = 'jumd'
                            "6332637300110010800000aa00389b71" // UUID
                            "03" // toggles
                            "633270612e7369676e617475726500" // label = "c2pa.signature"
                            // ---
                            "00000047" // box size
                            "75756964" // box type = 'uuid'
                            "6332637300110010800000aa00389b71"
                            "7468697320776f756c64206e6f726d61"
                            "6c6c792062652062696e617279207369"
                            "676e617475726520646174612e2e2e"
            );

            let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
            assert!(rem.is_empty());

            let claim_dbox = sbox
                .find_by_label("cb.adobe_1/c2pa.claim")
                .unwrap()
                .data_box()
                .unwrap();

            let sig_sbox = sbox
                .find_by_label("cb.adobe_1")
                .unwrap()
                .child_boxes
                .get(2)
                .unwrap();

            assert!(sig_sbox.as_data_box().is_none());

            let sig_sbox = sig_sbox.as_super_box().unwrap();
            assert!(claim_dbox.offset_within_superbox(sig_sbox).is_none());
        }
    }
}
