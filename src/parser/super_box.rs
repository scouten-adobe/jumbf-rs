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
    io::{Read, Seek, SeekFrom},
    rc::Rc,
};

use crate::{
    box_type::SUPER_BOX_TYPE,
    parser::{DataBox, DescriptionBox, Error, InputData, NoReader},
};

/// A JUMBF superbox contains a description box and zero or more
/// data boxes, each of which may or may not be a superbox.
///
/// The generic parameter `R` represents the reader type for lazy-loaded data.
#[derive(Clone, Eq, PartialEq)]
pub struct SuperBox<'a, R = NoReader> {
    /// Description box.
    pub desc: DescriptionBox<'a, R>,

    /// Child boxes. (These are referred to in some documentation
    /// as "data boxes.")
    pub child_boxes: Vec<ChildBox<'a, R>>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: InputData<'a, R>,
}

impl<'a> SuperBox<'a, NoReader> {
    /// Parse a byte-slice as a JUMBF superbox, and return a tuple of the parsed
    /// super box and the remainder of the input. Children of this superbox
    /// which are also superboxes will be parsed recursively without limit.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice(i: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        Self::from_slice_with_depth_limit(i, usize::MAX)
    }

    /// Parse a byte-slice as a JUMBF superbox, and return a tuple of the parsed
    /// super box and the remainder of the input. Children of this superbox
    /// which are also superboxes will be parsed recursively, to a limit of
    /// `depth_limit` nested boxes.
    ///
    /// If `depth_limit` is 0, any child superboxes that are found will be
    /// returned as plain [`DataBox`] structs instead.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_slice_with_depth_limit(
        i: &'a [u8],
        depth_limit: usize,
    ) -> Result<(Self, &'a [u8]), Error> {
        let (data_box, i) = DataBox::from_slice(i)?;
        let (sbox, _) = Self::from_data_box_with_depth_limit(&data_box, depth_limit)?;
        Ok((sbox, i))
    }

    /// Re-parse a [`DataBox`] as a JUMBF superbox. Children of this
    /// superbox which are also superboxes will be parsed recursively without
    /// limit.
    ///
    /// If the box is of `jumb` type and has the correct structure, returns
    /// a tuple of the new [`SuperBox`] object and the remainder of the input
    /// from the box (which should typically be empty).
    ///
    /// Will return an error if the box isn't of `jumb` type.
    pub fn from_data_box(data_box: &DataBox<'a, NoReader>) -> Result<(Self, &'a [u8]), Error> {
        Self::from_data_box_with_depth_limit(data_box, usize::MAX)
    }

    /// Re-parse a [`DataBox`] as a JUMBF superbox. Children of this superbox
    /// which are also superboxes will be parsed recursively, to a limit of
    /// `depth_limit` nested boxes.
    ///
    /// If the box is of `jumb` type and has the correct structure, returns
    /// a tuple of the new [`SuperBox`] object and the remainder of the input
    /// from the box (which should typically be empty). If `depth_limit` is 0,
    /// any child superboxes that are found will be returned as plain
    /// [`DataBox`] structs instead.
    ///
    /// Will return an error if the box isn't of `jumb` type.
    pub fn from_data_box_with_depth_limit(
        data_box: &DataBox<'a, NoReader>,
        depth_limit: usize,
    ) -> Result<(Self, &'a [u8]), Error> {
        if data_box.tbox != SUPER_BOX_TYPE {
            return Err(Error::InvalidSuperBoxType(data_box.tbox));
        }

        // Extract data as a slice.
        let data = match &data_box.data {
            InputData::Borrowed(slice) => *slice,
            InputData::Lazy(_) => {
                return Err(Error::IoError("Expected borrowed data".to_string()));
            }
        };

        let (desc, i) = DescriptionBox::from_slice(data)?;

        let (child_boxes, i) = boxes_from_slice(i)?;
        let child_boxes = child_boxes
            .into_iter()
            .map(|d| {
                if d.tbox == SUPER_BOX_TYPE && depth_limit > 0 {
                    let (sbox, _) = Self::from_data_box_with_depth_limit(&d, depth_limit - 1)?;
                    Ok(ChildBox::SuperBox(sbox))
                } else {
                    Ok(ChildBox::DataBox(d))
                }
            })
            .collect::<Result<Vec<ChildBox<'a, NoReader>>, Error>>()?;

        Ok((
            Self {
                desc,
                child_boxes,
                original: data_box.original.clone(),
            },
            i,
        ))
    }
}

impl<'a, R> SuperBox<'a, R> {
    /// Find a child superbox of this superbox by label and verify that
    /// exactly one such child exists.
    ///
    /// If label contains one or more slash (`/`) characters, the label
    /// will be treated as a hierarchical label and this function can then
    /// be used to traverse nested data structures.
    ///
    /// Will return `None` if no matching child superbox is found _or_ if
    /// more than one matching child superbox is found.
    pub fn find_by_label(&self, label: &str) -> Option<&Self> {
        let (label, suffix) = match label.split_once('/') {
            Some((label, suffix)) => (label, Some(suffix)),
            None => (label, None),
        };

        let matching_children: Vec<&SuperBox<'a, R>> = self
            .child_boxes
            .iter()
            .filter_map(|child_box| match child_box {
                ChildBox::SuperBox(sbox) => {
                    if let Some(ref sbox_label) = sbox.desc.label {
                        if sbox_label.as_str() == label && sbox.desc.requestable {
                            Some(sbox)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        if let Some(sbox) = matching_children.first() {
            if matching_children.len() == 1 {
                if let Some(suffix) = suffix {
                    sbox.find_by_label(suffix)
                } else {
                    Some(sbox)
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// If the first child box of this superbox is a data box, return it.
    /// Otherwise, return `None`.
    ///
    /// This is a convenience function for the common case where the superbox
    /// contains a non-superbox payload that needs to be interpreted further.
    pub fn data_box(&'a self) -> Option<&'a DataBox<'a, R>> {
        self.child_boxes
            .first()
            .and_then(|child_box| match child_box {
                ChildBox::DataBox(data_box) => Some(data_box),
                _ => None,
            })
    }
}

impl<'a, R: Debug> Debug for SuperBox<'a, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("SuperBox")
            .field("desc", &self.desc)
            .field("child_boxes", &self.child_boxes)
            .field("original", &self.original)
            .finish()
    }
}

impl<R: Read + Seek> SuperBox<'static, R> {
    /// Parse a JUMBF superbox from a reader at its current position.
    ///
    /// The reader position will be advanced to the end of the superbox upon
    /// success. Data is stored as lazy references and only read when
    /// `.to_vec()` is called. Children are parsed recursively without depth
    /// limit.
    pub fn from_reader(reader: Rc<RefCell<R>>) -> Result<Self, Error> {
        Self::from_reader_with_depth_limit(reader, usize::MAX)
    }

    /// Parse a JUMBF superbox from a reader at its current position with a
    /// depth limit.
    ///
    /// Children of this superbox which are also superboxes will be parsed
    /// recursively, to a limit of `depth_limit` nested boxes. If
    /// `depth_limit` is 0, any child superboxes that are found will be
    /// returned as plain [`DataBox`] structs instead.
    pub fn from_reader_with_depth_limit(
        reader: Rc<RefCell<R>>,
        depth_limit: usize,
    ) -> Result<Self, Error> {
        let data_box = DataBox::from_reader(Rc::clone(&reader))?;
        Self::from_data_box_with_depth_limit_reader(&data_box, depth_limit, reader)
    }

    fn from_data_box_with_depth_limit_reader(
        data_box: &DataBox<'static, R>,
        depth_limit: usize,
        reader: Rc<RefCell<R>>,
    ) -> Result<Self, Error> {
        if data_box.tbox != SUPER_BOX_TYPE {
            return Err(Error::InvalidSuperBoxType(data_box.tbox));
        }

        // The superbox's data field contains the description box and child boxes.
        // We need to seek to the start of the data and know where it ends.
        let (data_start_offset, data_end_offset) = match &data_box.data {
            InputData::Lazy(slice) => (slice.offset(), slice.offset() + slice.len() as u64),
            InputData::Borrowed(_) => {
                return Err(Error::IoError(
                    "Expected lazy data for reader-based parsing".to_string(),
                ));
            }
        };

        // Seek to the start of the superbox's data.
        reader
            .borrow_mut()
            .seek(SeekFrom::Start(data_start_offset))?;

        // Read description box from the superbox's data.
        let desc = DescriptionBox::from_reader(Rc::clone(&reader))?;

        // Read child boxes until we reach the end of the superbox's data.
        let child_boxes = boxes_from_reader(Rc::clone(&reader), data_end_offset)?;
        let child_boxes = child_boxes
            .into_iter()
            .map(|d| {
                if d.tbox == SUPER_BOX_TYPE && depth_limit > 0 {
                    let sbox = Self::from_data_box_with_depth_limit_reader(
                        &d,
                        depth_limit - 1,
                        Rc::clone(&reader),
                    )?;
                    Ok(ChildBox::SuperBox(sbox))
                } else {
                    Ok(ChildBox::DataBox(d))
                }
            })
            .collect::<Result<Vec<ChildBox<'static, R>>, Error>>()?;

        Ok(Self {
            desc,
            child_boxes,
            original: data_box.original.clone(),
        })
    }
}

// Parse boxes from slice until slice is empty.
fn boxes_from_slice(i: &[u8]) -> Result<(Vec<DataBox<'_>>, &[u8]), Error> {
    let mut result: Vec<DataBox> = vec![];
    let mut i = i;

    while !i.is_empty() {
        let (data_box, x) = DataBox::from_slice(i)?;
        i = x;
        result.push(data_box);
    }

    Ok((result, i))
}

// Parse boxes from reader until reader reaches the specified end offset.
fn boxes_from_reader<R: Read + Seek>(
    reader: Rc<RefCell<R>>,
    end_offset: u64,
) -> Result<Vec<DataBox<'static, R>>, Error> {
    let mut result: Vec<DataBox<'static, R>> = vec![];

    loop {
        let current_pos = reader.borrow_mut().stream_position()?;

        if current_pos >= end_offset {
            break;
        }

        let data_box = DataBox::from_reader(Rc::clone(&reader))?;
        result.push(data_box);
    }

    Ok(result)
}

/// This type represents a single box within a superbox,
/// which may itself be a superbox or or a regular box.
///
/// Note that this crate doesn't parse the content or ascribe
/// meaning to any type of box other than superbox (`jumb`) or
/// description box (`jumd`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildBox<'a, R = NoReader> {
    /// A superbox.
    SuperBox(SuperBox<'a, R>),

    /// Any other kind of box.
    DataBox(DataBox<'a, R>),
}

impl<'a, R> ChildBox<'a, R> {
    /// If this represents a nested super box, return a reference to that
    /// superbox.
    pub fn as_super_box(&'a self) -> Option<&'a SuperBox<'a, R>> {
        if let Self::SuperBox(sb) = self {
            Some(sb)
        } else {
            None
        }
    }

    /// If this represents a nested data box, return a reference to that data
    /// box.
    pub fn as_data_box(&'a self) -> Option<&'a DataBox<'a, R>> {
        if let Self::DataBox(db) = self {
            Some(db)
        } else {
            None
        }
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
        parser::{ChildBox, DataBox, DescriptionBox, Error, InputData, Label, SuperBox},
        BoxType,
    };

    #[test]
    fn simple_super_box() {
        let jumbf = hex!(
            "0000002f" // box size
            "6a756d62" // box type = 'jumb'
                "00000027" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e7375706572626f7800" // label
        );

        let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: Some(Label::Borrowed("test.superbox")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..47]),
                },
                child_boxes: vec!(),
                original: InputData::Borrowed(&jumbf),
            }
        );
    }

    #[test]
    fn child_superbox_without_label() {
        let jumbf = hex!(
            "00000058" // box size
            "6a756d62" // box type = 'jumb'
                "0000002f" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e7375706572626f785f64617461626f7800" // label
                // ------
                "00000021" // box size
                "6a756d62" // box type = 'jumb'
                    "00000019" // box size
                    "6a756d64" // box type = 'jumbd'
                    "00000000000000000000000000000000" // UUID
                    "00" // toggles
        );

        let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: Some(Label::Borrowed("test.superbox_databox")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..55]),
                },
                child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: [0; 16],
                        label: None,
                        requestable: false,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&jumbf[63..88]),
                    },
                    child_boxes: vec!(),
                    original: InputData::Borrowed(&jumbf[55..88]),
                })),
                original: InputData::Borrowed(&jumbf),
            }
        );

        assert!(sbox.find_by_label("not_there").is_none());

        let dbox_as_child = sbox.child_boxes.first().unwrap();
        assert!(dbox_as_child.as_data_box().is_none());
        assert_eq!(
            dbox_as_child.as_super_box().unwrap(),
            &SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: None,
                    requestable: false,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[63..88]),
                },
                child_boxes: vec!(),
                original: InputData::Borrowed(&jumbf[55..88]),
            }
        );
    }

    #[test]
    fn data_box_sample() {
        let jumbf = hex!(
        "00000077" // box size
        "6a756d62" // box type = 'jumb'
            "00000028" // box size
            "6a756d64" // box type = 'jumd'
            "6332637300110010800000aa00389b71" // UUID
            "03" // toggles
            "633270612e7369676e617475726500" // label
            // ----
            "00000047" // box size
            "75756964" // box type = 'uuid'
            "6332637300110010800000aa00389b717468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e" // data (type unknown)
        );

        let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113],
                    label: Some(Label::Borrowed("c2pa.signature")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..48]),
                },
                child_boxes: vec!(ChildBox::DataBox(DataBox {
                    tbox: BoxType(*b"uuid"),
                    data: InputData::Borrowed(&[
                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104,
                        105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108,
                        108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103,
                        110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                    ]),
                    original: InputData::Borrowed(&jumbf[48..119]),
                })),
                original: InputData::Borrowed(&jumbf),
            }
        );

        let uuid_box = sbox.data_box().unwrap();
        assert_eq!(uuid_box.offset_within_superbox(&sbox).unwrap(), 56);

        let dbox_as_child = sbox.child_boxes.first().unwrap();
        assert!(dbox_as_child.as_super_box().is_none());
        assert_eq!(
            dbox_as_child.as_data_box().unwrap(),
            &DataBox {
                tbox: BoxType(*b"uuid"),
                data: InputData::Borrowed(&[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ]),
                original: InputData::Borrowed(&jumbf[48..119]),
            }
        );
    }

    #[test]
    fn complex_example() {
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

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some(Label::Borrowed("c2pa")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..38]),
                },
                child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("cb.adobe_1")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&jumbf[46..82]),
                    },
                    child_boxes: vec!(
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.assertions")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[90..131]),
                            },
                            child_boxes: vec![ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56,
                                        155, 113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.location.broad")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&jumbf[139..184]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"json"),
                                    data: InputData::Borrowed(&[
                                        123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58,
                                        32, 34, 77, 97, 114, 103, 97, 116, 101, 32, 67, 105, 116,
                                        121, 44, 32, 78, 74, 34, 125,
                                    ]),
                                    original: InputData::Borrowed(&jumbf[184..225]),
                                },),],
                                original: InputData::Borrowed(&jumbf[131..225]),
                            },),],
                            original: InputData::Borrowed(&jumbf[82..225]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.claim")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[233..269]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"json"),
                                data: InputData::Borrowed(&[
                                    123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34,
                                    114, 101, 99, 111, 114, 100, 101, 114, 34, 32, 58, 32, 34, 80,
                                    104, 111, 116, 111, 115, 104, 111, 112, 34, 44, 10, 32, 32, 32,
                                    32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 115, 105, 103, 110, 97,
                                    116, 117, 114, 101, 34, 32, 58, 32, 34, 115, 101, 108, 102, 35,
                                    106, 117, 109, 98, 102, 61, 115, 95, 97, 100, 111, 98, 101, 95,
                                    49, 34, 44, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                    34, 97, 115, 115, 101, 114, 116, 105, 111, 110, 115, 34, 32,
                                    58, 32, 91, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                    32, 32, 32, 32, 34, 115, 101, 108, 102, 35, 106, 117, 109, 98,
                                    102, 61, 97, 115, 95, 97, 100, 111, 98, 101, 95, 49, 47, 99,
                                    50, 112, 97, 46, 108, 111, 99, 97, 116, 105, 111, 110, 46, 98,
                                    114, 111, 97, 100, 63, 104, 108, 61, 55, 54, 49, 52, 50, 66,
                                    68, 54, 50, 51, 54, 51, 70, 34, 10, 32, 32, 32, 32, 32, 32, 32,
                                    32, 32, 32, 32, 32, 93, 10, 32, 32, 32, 32, 32, 32, 32, 32,
                                    125,
                                ]),
                                original: InputData::Borrowed(&jumbf[269..496]),
                            },),],
                            original: InputData::Borrowed(&jumbf[225..496]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.signature")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[504..544]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"uuid"),
                                data: InputData::Borrowed(&[
                                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                    116, 104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111,
                                    114, 109, 97, 108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97,
                                    114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114, 101, 32,
                                    100, 97, 116, 97, 46, 46, 46,
                                ]),
                                original: InputData::Borrowed(&jumbf[544..615]),
                            },),],
                            original: InputData::Borrowed(&jumbf[496..615]),
                        },),
                    ),
                    original: InputData::Borrowed(&jumbf[38..615]),
                })),
                original: InputData::Borrowed(&jumbf),
            }
        );

        assert_eq!(
            sbox.find_by_label("cb.adobe_1/c2pa.signature"),
            Some(&SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some(Label::Borrowed("c2pa.signature")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[504..544]),
                },
                child_boxes: vec![ChildBox::DataBox(DataBox {
                    tbox: BoxType(*b"uuid"),
                    data: InputData::Borrowed(&[
                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104,
                        105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108,
                        108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103,
                        110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                    ]),
                    original: InputData::Borrowed(&jumbf[544..615]),
                },),],
                original: InputData::Borrowed(&jumbf[496..615]),
            })
        );

        assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
        assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
        assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

        assert_eq!(
            sbox.find_by_label("cb.adobe_1/c2pa.signature")
                .and_then(|sig| sig.data_box()),
            Some(&DataBox {
                tbox: BoxType(*b"uuid"),
                data: InputData::Borrowed(&[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ]),
                original: InputData::Borrowed(&jumbf[544..615]),
            })
        );

        assert_eq!(
            sbox.find_by_label("cb.adobe_1/c2pa.signature")
                .and_then(|sig| sig.data_box())
                .and_then(|sig| sig.offset_within_superbox(&sbox))
                .unwrap(),
            552
        );

        assert_eq!(sbox.data_box(), None);
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
            SuperBox::from_slice(&jumbf).unwrap_err(),
            Error::InvalidSuperBoxType(BoxType(*b"jumc"))
        );
    }

    #[test]
    fn find_by_label_avoids_confict() {
        let jumbf = hex!(
            "00000093" // box size
            "6a756d62" // box type = 'jumb'
                "0000002f" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e7375706572626f785f64617461626f7800" // label
                // ------
                "0000002e" // box size
                "6a756d62" // box type = 'jumb'
                    "00000026" // box size
                    "6a756d64" // box type = 'jumbd'
                    "00000000000000000000000000000000" // UUID
                    "03" // toggles
                    "746573742e64617461626f7800" // label = "test.databox"
                // ------
                "0000002e" // box size
                "6a756d62" // box type = 'jumb'
                    "00000026" // box size
                    "6a756d64" // box type = 'jumbd'
                    "00000000000000000000000000000000" // UUID
                    "03" // toggles
                    "746573742e64617461626f7800" // label = "test.databox"
        );

        let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: Some(Label::Borrowed("test.superbox_databox")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..55]),
                },
                child_boxes: vec!(
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [0; 16],
                            label: Some(Label::Borrowed("test.databox")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&jumbf[63..101]),
                        },
                        child_boxes: vec!(),
                        original: InputData::Borrowed(&jumbf[55..101]),
                    }),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [0; 16],
                            label: Some(Label::Borrowed("test.databox")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&jumbf[109..147]),
                        },
                        child_boxes: vec!(),
                        original: InputData::Borrowed(&jumbf[101..147]),
                    })
                ),
                original: InputData::Borrowed(&jumbf),
            }
        );

        assert_eq!(sbox.find_by_label("test.databox"), None);
    }

    #[test]
    fn find_by_label_skips_non_requestable_boxes() {
        let jumbf = hex!(
            "00000093" // box size
            "6a756d62" // box type = 'jumb'
                "0000002f" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e7375706572626f785f64617461626f7800" // label
                // ------
                "0000002e" // box size
                "6a756d62" // box type = 'jumb'
                    "00000026" // box size
                    "6a756d64" // box type = 'jumbd'
                    "00000000000000000000000000000000" // UUID
                    "02" // toggles
                    "746573742e64617461626f7800" // label = "test.databox"
                // ------
                "0000002e" // box size
                "6a756d62" // box type = 'jumb'
                    "00000026" // box size
                    "6a756d64" // box type = 'jumbd'
                    "00000000000000000000000000000000" // UUID
                    "03" // toggles
                    "746573742e64617461626f7a00" // label = "test.databoz"
        );

        let (sbox, rem) = SuperBox::from_slice(&jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: Some(Label::Borrowed("test.superbox_databox")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..55]),
                },
                child_boxes: vec!(
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [0; 16],
                            label: Some(Label::Borrowed("test.databox")),
                            requestable: false,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&jumbf[63..101]),
                        },
                        child_boxes: vec!(),
                        original: InputData::Borrowed(&jumbf[55..101]),
                    }),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [0; 16],
                            label: Some(Label::Borrowed("test.databoz")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&jumbf[109..147]),
                        },
                        child_boxes: vec!(),
                        original: InputData::Borrowed(&jumbf[101..147]),
                    })
                ),
                original: InputData::Borrowed(&jumbf),
            }
        );

        assert_eq!(sbox.find_by_label("test.databox"), None);

        assert_eq!(
            sbox.find_by_label("test.databoz"),
            Some(&SuperBox {
                desc: DescriptionBox {
                    uuid: [0; 16],
                    label: Some(Label::Borrowed("test.databoz")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[109..147]),
                },
                child_boxes: vec!(),
                original: InputData::Borrowed(&jumbf[101..147]),
            })
        );
    }

    #[test]
    fn parse_c2pa_manifest() {
        let jumbf = include_bytes!("../tests/fixtures/C.c2pa");

        let (sbox, rem) = SuperBox::from_slice(jumbf).unwrap();
        assert!(rem.is_empty());

        assert_eq!(
            sbox,
            SuperBox {
                desc: DescriptionBox {
                    uuid: hex!("63 32 70 61 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                    label: Some(Label::Borrowed("c2pa")),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: InputData::Borrowed(&jumbf[8..38]),
                },
                child_boxes: vec![ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: hex!("63 32 6d 61 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                        label: Some(Label::Borrowed(
                            "contentauth:urn:uuid:021b555e-5e02-4074-b444-43d7919d89b9"
                        )),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&jumbf[46..129]),
                    },
                    child_boxes: vec![
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: hex!("63 32 61 73 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                label: Some(Label::Borrowed("c2pa.assertions")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[137..178]),
                            },
                            child_boxes: vec![
                                ChildBox::SuperBox(SuperBox {
                                    desc: DescriptionBox {
                                        uuid: hex!(
                                            "40 cb 0c 32 bb 8a 48 9d a7 0b 2a d6 f4 7f 43 69"
                                        ),
                                        label: Some(Label::Borrowed("c2pa.thumbnail.claim.jpeg")),
                                        requestable: true,
                                        id: None,
                                        hash: None,
                                        private: None,
                                        original: InputData::Borrowed(&jumbf[186..237]),
                                    },
                                    child_boxes: vec![
                                        ChildBox::DataBox(DataBox {
                                            tbox: BoxType(*b"bfdb"),
                                            data: InputData::Borrowed(&jumbf[245..257]),
                                            original: InputData::Borrowed(&jumbf[237..257]),
                                        },),
                                        ChildBox::DataBox(DataBox {
                                            tbox: BoxType(*b"bidb"),
                                            data: InputData::Borrowed(&jumbf[265..31976]),
                                            original: InputData::Borrowed(&jumbf[257..31976]),
                                        },),
                                    ],
                                    original: InputData::Borrowed(&jumbf[178..31976]),
                                },),
                                ChildBox::SuperBox(SuperBox {
                                    desc: DescriptionBox {
                                        uuid: hex!(
                                            "6a 73 6f 6e 00 11 00 10 80 00 00 aa 00 38 9b 71"
                                        ),
                                        label: Some(Label::Borrowed(
                                            "stds.schema-org.CreativeWork"
                                        )),
                                        requestable: true,
                                        id: None,
                                        hash: None,
                                        private: Some(DataBox {
                                            tbox: BoxType(*b"c2sh"),
                                            data: InputData::Borrowed(&jumbf[32046..32062]),
                                            original: InputData::Borrowed(&jumbf[32038..32062]),
                                        },),
                                        original: InputData::Borrowed(&jumbf[31984..32062]),
                                    },
                                    child_boxes: vec![ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"json"),
                                        data: InputData::Borrowed(&jumbf[32070..32179]),
                                        original: InputData::Borrowed(&jumbf[32062..32179]),
                                    },),],
                                    original: InputData::Borrowed(&jumbf[31976..32179]),
                                },),
                                ChildBox::SuperBox(SuperBox {
                                    desc: DescriptionBox {
                                        uuid: hex!(
                                            "63 62 6f 72 00 11 00 10 80 00 00 aa 00 38 9b 71"
                                        ),
                                        label: Some(Label::Borrowed("c2pa.actions")),
                                        requestable: true,
                                        id: None,
                                        hash: None,
                                        private: None,
                                        original: InputData::Borrowed(&jumbf[32187..32225]),
                                    },
                                    child_boxes: vec![ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"cbor"),
                                        data: InputData::Borrowed(&jumbf[32233..32311]),
                                        original: InputData::Borrowed(&jumbf[32225..32311]),
                                    },),],
                                    original: InputData::Borrowed(&jumbf[32179..32311]),
                                },),
                                ChildBox::SuperBox(SuperBox {
                                    desc: DescriptionBox {
                                        uuid: hex!(
                                            "63 62 6f 72 00 11 00 10 80 00 00 aa 00 38 9b 71"
                                        ),
                                        label: Some(Label::Borrowed("c2pa.hash.data")),
                                        requestable: true,
                                        id: None,
                                        hash: None,
                                        private: None,
                                        original: InputData::Borrowed(&jumbf[32319..32359]),
                                    },
                                    child_boxes: vec![ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"cbor"),
                                        data: InputData::Borrowed(&jumbf[32367..32482]),
                                        original: InputData::Borrowed(&jumbf[32359..32482]),
                                    },),],
                                    original: InputData::Borrowed(&jumbf[32311..32482]),
                                },),
                            ],
                            original: InputData::Borrowed(&jumbf[129..32482]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: hex!("63 32 63 6c 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                label: Some(Label::Borrowed("c2pa.claim")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[32490..32526]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"cbor"),
                                data: InputData::Borrowed(&jumbf[32534..33166]),
                                original: InputData::Borrowed(&jumbf[32526..33166]),
                            },),],
                            original: InputData::Borrowed(&jumbf[32482..33166]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: hex!("63 32 63 73 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                label: Some(Label::Borrowed("c2pa.signature")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&jumbf[33174..33214]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"cbor"),
                                data: InputData::Borrowed(&jumbf[33222..46948]),
                                original: InputData::Borrowed(&jumbf[33214..46948]),
                            },),],
                            original: InputData::Borrowed(&jumbf[33166..46948]),
                        },),
                    ],
                    original: InputData::Borrowed(&jumbf[38..46948]),
                },),],
                original: InputData::Borrowed(&jumbf[0..46948]),
            }
        );
    }

    mod depth_limit {
        use hex_literal::hex;
        use pretty_assertions_sorted::assert_eq;

        use crate::{
            parser::{ChildBox, DataBox, DescriptionBox, InputData, Label, SuperBox},
            BoxType,
        };

        const JUMBF: [u8; 615] = hex!(
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

        #[test]
        fn depth_limit_0() {
            let (sbox, rem) = SuperBox::from_slice_with_depth_limit(&JUMBF, 0).unwrap();
            assert!(rem.is_empty());

            assert_eq!(
                sbox,
                SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[8..38]),
                    },
                    child_boxes: vec!(ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"jumb"),
                        original: InputData::Borrowed(&JUMBF[38..615]),
                        data: InputData::Borrowed(&JUMBF[46..615]),
                    })),
                    original: InputData::Borrowed(&JUMBF),
                }
            );

            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

            let data_box = sbox.data_box().unwrap();

            assert_eq!(
                data_box,
                &DataBox {
                    tbox: BoxType(*b"jumb"),
                    original: InputData::Borrowed(&JUMBF[38..615]),
                    data: InputData::Borrowed(&JUMBF[46..615]),
                }
            );

            let (nested_box, _) = SuperBox::from_data_box(data_box).unwrap();

            assert_eq!(
                nested_box,
                SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("cb.adobe_1")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[46..82]),
                    },
                    child_boxes: vec!(
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.assertions")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&JUMBF[90..131]),
                            },
                            child_boxes: vec![ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56,
                                        155, 113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.location.broad")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[139..184]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"json"),
                                    data: InputData::Borrowed(&[
                                        123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58,
                                        32, 34, 77, 97, 114, 103, 97, 116, 101, 32, 67, 105, 116,
                                        121, 44, 32, 78, 74, 34, 125,
                                    ]),
                                    original: InputData::Borrowed(&JUMBF[184..225]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[131..225]),
                            },),],
                            original: InputData::Borrowed(&JUMBF[82..225]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.claim")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&JUMBF[233..269]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"json"),
                                data: InputData::Borrowed(&[
                                    123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34,
                                    114, 101, 99, 111, 114, 100, 101, 114, 34, 32, 58, 32, 34, 80,
                                    104, 111, 116, 111, 115, 104, 111, 112, 34, 44, 10, 32, 32, 32,
                                    32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 115, 105, 103, 110, 97,
                                    116, 117, 114, 101, 34, 32, 58, 32, 34, 115, 101, 108, 102, 35,
                                    106, 117, 109, 98, 102, 61, 115, 95, 97, 100, 111, 98, 101, 95,
                                    49, 34, 44, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                    34, 97, 115, 115, 101, 114, 116, 105, 111, 110, 115, 34, 32,
                                    58, 32, 91, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                    32, 32, 32, 32, 34, 115, 101, 108, 102, 35, 106, 117, 109, 98,
                                    102, 61, 97, 115, 95, 97, 100, 111, 98, 101, 95, 49, 47, 99,
                                    50, 112, 97, 46, 108, 111, 99, 97, 116, 105, 111, 110, 46, 98,
                                    114, 111, 97, 100, 63, 104, 108, 61, 55, 54, 49, 52, 50, 66,
                                    68, 54, 50, 51, 54, 51, 70, 34, 10, 32, 32, 32, 32, 32, 32, 32,
                                    32, 32, 32, 32, 32, 93, 10, 32, 32, 32, 32, 32, 32, 32, 32,
                                    125,
                                ]),
                                original: InputData::Borrowed(&JUMBF[269..496]),
                            },),],
                            original: InputData::Borrowed(&JUMBF[225..496]),
                        },),
                        ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                ],
                                label: Some(Label::Borrowed("c2pa.signature")),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: InputData::Borrowed(&JUMBF[504..544]),
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"uuid"),
                                data: InputData::Borrowed(&[
                                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                    116, 104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111,
                                    114, 109, 97, 108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97,
                                    114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114, 101, 32,
                                    100, 97, 116, 97, 46, 46, 46,
                                ]),
                                original: InputData::Borrowed(&JUMBF[544..615]),
                            },),],
                            original: InputData::Borrowed(&JUMBF[496..615]),
                        },),
                    ),
                    original: InputData::Borrowed(&JUMBF[38..615]),
                }
            );
        }

        #[test]
        fn depth_limit_1() {
            let (sbox, rem) = SuperBox::from_slice_with_depth_limit(&JUMBF, 1).unwrap();
            assert!(rem.is_empty());

            assert_eq!(
                sbox,
                SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[8..38]),
                    },
                    child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some(Label::Borrowed("cb.adobe_1")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&JUMBF[46..82]),
                        },
                        child_boxes: vec!(
                            ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"jumb"),
                                original: InputData::Borrowed(&JUMBF[82..225]),
                                data: InputData::Borrowed(&JUMBF[90..225]),
                            }),
                            ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"jumb"),
                                original: InputData::Borrowed(&JUMBF[225..496]),
                                data: InputData::Borrowed(&JUMBF[233..496]),
                            }),
                            ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"jumb"),
                                original: InputData::Borrowed(&JUMBF[496..615]),
                                data: InputData::Borrowed(&JUMBF[504..615]),
                            }),
                        ),
                        original: InputData::Borrowed(&JUMBF[38..615]),
                    })),
                    original: InputData::Borrowed(&JUMBF),
                }
            );

            assert!(sbox.find_by_label("cb.adobe_1").is_some());
            assert!(sbox.find_by_label("cb.adobe_1/c2pa.signature").is_none());
            assert!(sbox.data_box().is_none());
        }

        #[test]
        fn depth_limit_2() {
            let (sbox, rem) = SuperBox::from_slice_with_depth_limit(&JUMBF, 2).unwrap();
            assert!(rem.is_empty());

            assert_eq!(
                sbox,
                SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[8..38]),
                    },
                    child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some(Label::Borrowed("cb.adobe_1")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&JUMBF[46..82]),
                        },
                        child_boxes: vec!(
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.assertions")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[90..131]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"jumb"),
                                    data: InputData::Borrowed(&JUMBF[139..225]),
                                    original: InputData::Borrowed(&JUMBF[131..225]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[82..225]),
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.claim")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[233..269]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"json"),
                                    data: InputData::Borrowed(&[
                                        123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                        34, 114, 101, 99, 111, 114, 100, 101, 114, 34, 32, 58, 32,
                                        34, 80, 104, 111, 116, 111, 115, 104, 111, 112, 34, 44, 10,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 115,
                                        105, 103, 110, 97, 116, 117, 114, 101, 34, 32, 58, 32, 34,
                                        115, 101, 108, 102, 35, 106, 117, 109, 98, 102, 61, 115,
                                        95, 97, 100, 111, 98, 101, 95, 49, 34, 44, 10, 32, 32, 32,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 97, 115, 115, 101,
                                        114, 116, 105, 111, 110, 115, 34, 32, 58, 32, 91, 10, 32,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                        34, 115, 101, 108, 102, 35, 106, 117, 109, 98, 102, 61, 97,
                                        115, 95, 97, 100, 111, 98, 101, 95, 49, 47, 99, 50, 112,
                                        97, 46, 108, 111, 99, 97, 116, 105, 111, 110, 46, 98, 114,
                                        111, 97, 100, 63, 104, 108, 61, 55, 54, 49, 52, 50, 66, 68,
                                        54, 50, 51, 54, 51, 70, 34, 10, 32, 32, 32, 32, 32, 32, 32,
                                        32, 32, 32, 32, 32, 93, 10, 32, 32, 32, 32, 32, 32, 32, 32,
                                        125,
                                    ]),
                                    original: InputData::Borrowed(&JUMBF[269..496]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[225..496]),
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.signature")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[504..544]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"uuid"),
                                    data: InputData::Borrowed(&[
                                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113, 116, 104, 105, 115, 32, 119, 111, 117, 108, 100, 32,
                                        110, 111, 114, 109, 97, 108, 108, 121, 32, 98, 101, 32, 98,
                                        105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                                        117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                                    ]),
                                    original: InputData::Borrowed(&JUMBF[544..615]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[496..615]),
                            },),
                        ),
                        original: InputData::Borrowed(&JUMBF[38..615]),
                    })),
                    original: InputData::Borrowed(&JUMBF),
                }
            );

            assert_eq!(
                sbox.find_by_label("cb.adobe_1/c2pa.signature"),
                Some(&SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa.signature")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[504..544]),
                    },
                    child_boxes: vec![ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"uuid"),
                        data: InputData::Borrowed(&[
                            99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116,
                            104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97,
                            108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115,
                            105, 103, 110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46,
                            46,
                        ]),
                        original: InputData::Borrowed(&JUMBF[544..615]),
                    },),],
                    original: InputData::Borrowed(&JUMBF[496..615]),
                })
            );

            assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

            assert_eq!(
                sbox.find_by_label("cb.adobe_1/c2pa.signature")
                    .and_then(|sig| sig.data_box()),
                Some(&DataBox {
                    tbox: BoxType(*b"uuid"),
                    data: InputData::Borrowed(&[
                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104,
                        105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108,
                        108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103,
                        110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                    ]),
                    original: InputData::Borrowed(&JUMBF[544..615]),
                })
            );

            assert_eq!(sbox.data_box(), None);
        }

        #[test]
        fn depth_limit_3() {
            let (sbox, rem) = SuperBox::from_slice_with_depth_limit(&JUMBF, 3).unwrap();
            assert!(rem.is_empty());

            assert_eq!(
                sbox,
                SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[8..38]),
                    },
                    child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some(Label::Borrowed("cb.adobe_1")),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: InputData::Borrowed(&JUMBF[46..82]),
                        },
                        child_boxes: vec!(
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.assertions")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[90..131]),
                                },
                                child_boxes: vec![ChildBox::SuperBox(SuperBox {
                                    desc: DescriptionBox {
                                        uuid: [
                                            106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0,
                                            56, 155, 113,
                                        ],
                                        label: Some(Label::Borrowed("c2pa.location.broad")),
                                        requestable: true,
                                        id: None,
                                        hash: None,
                                        private: None,
                                        original: InputData::Borrowed(&JUMBF[139..184]),
                                    },
                                    child_boxes: vec![ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"json"),
                                        data: InputData::Borrowed(&[
                                            123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34,
                                            58, 32, 34, 77, 97, 114, 103, 97, 116, 101, 32, 67,
                                            105, 116, 121, 44, 32, 78, 74, 34, 125,
                                        ]),
                                        original: InputData::Borrowed(&JUMBF[184..225]),
                                    },),],
                                    original: InputData::Borrowed(&JUMBF[131..225]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[82..225]),
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.claim")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[233..269]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"json"),
                                    data: InputData::Borrowed(&[
                                        123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                        34, 114, 101, 99, 111, 114, 100, 101, 114, 34, 32, 58, 32,
                                        34, 80, 104, 111, 116, 111, 115, 104, 111, 112, 34, 44, 10,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 115,
                                        105, 103, 110, 97, 116, 117, 114, 101, 34, 32, 58, 32, 34,
                                        115, 101, 108, 102, 35, 106, 117, 109, 98, 102, 61, 115,
                                        95, 97, 100, 111, 98, 101, 95, 49, 34, 44, 10, 32, 32, 32,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 97, 115, 115, 101,
                                        114, 116, 105, 111, 110, 115, 34, 32, 58, 32, 91, 10, 32,
                                        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                                        34, 115, 101, 108, 102, 35, 106, 117, 109, 98, 102, 61, 97,
                                        115, 95, 97, 100, 111, 98, 101, 95, 49, 47, 99, 50, 112,
                                        97, 46, 108, 111, 99, 97, 116, 105, 111, 110, 46, 98, 114,
                                        111, 97, 100, 63, 104, 108, 61, 55, 54, 49, 52, 50, 66, 68,
                                        54, 50, 51, 54, 51, 70, 34, 10, 32, 32, 32, 32, 32, 32, 32,
                                        32, 32, 32, 32, 32, 93, 10, 32, 32, 32, 32, 32, 32, 32, 32,
                                        125,
                                    ]),
                                    original: InputData::Borrowed(&JUMBF[269..496]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[225..496]),
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: [
                                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113,
                                    ],
                                    label: Some(Label::Borrowed("c2pa.signature")),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: InputData::Borrowed(&JUMBF[504..544]),
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"uuid"),
                                    data: InputData::Borrowed(&[
                                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                        113, 116, 104, 105, 115, 32, 119, 111, 117, 108, 100, 32,
                                        110, 111, 114, 109, 97, 108, 108, 121, 32, 98, 101, 32, 98,
                                        105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                                        117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                                    ]),
                                    original: InputData::Borrowed(&JUMBF[544..615]),
                                },),],
                                original: InputData::Borrowed(&JUMBF[496..615]),
                            },),
                        ),
                        original: InputData::Borrowed(&JUMBF[38..615]),
                    })),
                    original: InputData::Borrowed(&JUMBF),
                }
            );

            assert_eq!(
                sbox.find_by_label("cb.adobe_1/c2pa.signature"),
                Some(&SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some(Label::Borrowed("c2pa.signature")),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: InputData::Borrowed(&JUMBF[504..544]),
                    },
                    child_boxes: vec![ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"uuid"),
                        data: InputData::Borrowed(&[
                            99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116,
                            104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97,
                            108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115,
                            105, 103, 110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46,
                            46,
                        ]),
                        original: InputData::Borrowed(&JUMBF[544..615]),
                    },),],
                    original: InputData::Borrowed(&JUMBF[496..615]),
                })
            );

            assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
            assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

            assert_eq!(
                sbox.find_by_label("cb.adobe_1/c2pa.signature")
                    .and_then(|sig| sig.data_box()),
                Some(&DataBox {
                    tbox: BoxType(*b"uuid"),
                    data: InputData::Borrowed(&[
                        99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104,
                        105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108,
                        108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103,
                        110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                    ]),
                    original: InputData::Borrowed(&JUMBF[544..615]),
                })
            );

            assert_eq!(sbox.data_box(), None);
        }
    }
}
