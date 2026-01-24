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

use std::io::Result;

use crate::{
    box_type::{DESCRIPTION_BOX_TYPE, SUPER_BOX_TYPE},
    builder::{
        to_box::{jumbf_size, write_jumbf},
        ToBox, WriteAndSeek,
    },
    BoxType,
};

/// A `SuperBoxBuilder` helps you create a JUMBF superbox which contains zero or
/// more child boxes, each of which may or may not be a superbox.
///
/// Construct a superbox by calling [`SuperBoxBuilder::new()`] and then calling
/// one or more methods on this struct to describe the box and its contents.
///
/// Use [`SuperBoxBuilder::add_child_box()`] as many times as needed to add
/// children of this box. Any data type which implements [`ToBox`] (including
/// `SuperBoxBuilder` itself) may be used.
///
/// The JUMBF specification requires that the first child box of any superbox
/// is a "description box" which describes the superbox and its content.
/// The description box is generated automatically; you should not explicitly
/// create one.
///
/// When done, call [`SuperBoxBuilder::write_jumbf()`] to convert the superbox
/// to a JUMBF byte stream.
///
/// ## Example
///
/// ```
/// # fn example() -> std::io::Result<()> {
/// use std::io::Cursor;
///
/// use hex_literal::hex;
/// use jumbf::{BoxType, builder::{DataBoxBuilder, SuperBoxBuilder}};
///
/// let dbox = DataBoxBuilder::from_borrowed(BoxType(*b"abcd"), b"some data");
/// let uuid = [0u8; 16]; // replace with your app-specific UUID
///
/// let sbox = SuperBoxBuilder::new(&uuid).add_child_box(dbox);
///
/// let mut jumbf = Cursor::new(Vec::<u8>::new());
/// sbox.write_jumbf(&mut jumbf)?;
///
/// let expected_jumbf = hex!(
///     "0000004a" // box size
///     "6a756d62" // box type = 'jumb'
///         "00000019" // box size
///         "6a756d64" // box type = 'jumd'
///         "00000000000000000000000000000000" // UUID
///         "00" // toggles
///         // ---
///         "00000029" // box size
///         "61626364" // box type = 'abcd'
///         "736f6d652064617461" // payload ("some data")
///     );
///
/// assert_eq!(*jumbf.into_inner(), expected_jumbf);
/// # Ok(())
/// # }
/// ```
pub struct SuperBoxBuilder<'a> {
    desc: DescriptionBoxBuilder,
    child_boxes: Vec<OwnedOrBorrowedBox<'a>>,
}

impl<'a> SuperBoxBuilder<'a> {
    /// Create a new, empty superbox.
    ///
    /// A superbox is identified by an application-specific UUID.
    /// This crate does not interpret the UUID. Any 16-byte
    /// value is allowed.
    pub fn new(uuid: &[u8; 16]) -> Self {
        Self {
            desc: DescriptionBoxBuilder::new(uuid),
            child_boxes: vec![],
        }
    }

    /// Set an application-specific label for the superbox.
    ///
    /// This label will flagged as "requestable," meaning a search via
    /// [`SuperBox::find_by_label()`] or an equivalent function in another
    /// JUMBF parser with this label should return it.
    ///
    /// If, for some reason, that is not desired, you can use
    /// [`set_non_requestable_label()`] instead.
    ///
    /// [`SuperBox::find_by_label()`]: crate::parser::SuperBox::find_by_label()
    /// [`set_non_requestable_label()`]: Self::set_non_requestable_label()
    pub fn set_label<S: AsRef<str>>(mut self, label: S) -> Self {
        self.desc.label = Some(label.as_ref().to_owned());
        self.desc.requestable = true;
        self
    }

    /// Set an application-specific label for the superbox.
    ///
    /// This label is flagged as non-requestable, meaning a search via
    /// [`SuperBox::find_by_label()`] or an equivalent function in another
    /// JUMBF parser should not return it.
    ///
    /// [`SuperBox::find_by_label()`]: crate::parser::SuperBox::find_by_label()
    pub fn set_non_requestable_label<S: AsRef<str>>(mut self, label: S) -> Self {
        self.desc.label = Some(label.as_ref().to_owned());
        self.desc.requestable = false;
        self
    }

    /// Set an application-specific 32-bit ID.
    pub fn set_id(mut self, id: u32) -> Self {
        self.desc.id = Some(id);
        self
    }

    /// Provide a SHA-256 has for this superbox's data payload.
    ///
    /// Note that this crate does not verify the correctness of
    /// this hash.
    pub fn set_sha256_hash(mut self, hash: &[u8; 32]) -> Self {
        self.desc.hash = Some(*hash);
        self
    }

    /// Provide an application-specific "private" box within
    /// the description box. Takes ownership of the box.
    pub fn set_private_box(mut self, private: impl ToBox + 'static) -> Self {
        self.desc.private = Some(Box::new(private));
        self
    }

    /// Add a child box. Takes ownership of the box.
    pub fn add_child_box(mut self, boxx: impl ToBox + 'static) -> Self {
        self.child_boxes
            .push(OwnedOrBorrowedBox::OwnedBox(Box::new(boxx)));
        self
    }

    /// Add a child box without taking ownership.
    ///
    /// The child box's lifetime must be at least as long as this superbox.
    pub fn add_borrowed_child_box<B: ToBox>(mut self, boxx: &'a B) -> Self {
        self.child_boxes.push(OwnedOrBorrowedBox::BorrowedBox(boxx));
        self
    }

    /// Write this superbox and all of its child boxes to a JUMBF stream.
    pub fn write_jumbf(&self, to_stream: &mut dyn WriteAndSeek) -> Result<()> {
        write_jumbf(self, to_stream)
    }
}

impl<'a> ToBox for SuperBoxBuilder<'a> {
    fn box_type(&self) -> BoxType {
        SUPER_BOX_TYPE
    }

    fn payload_size(&self) -> Result<usize> {
        let mut size: usize = jumbf_size(&self.desc)?;

        for child in &self.child_boxes {
            size += jumbf_size(child.as_ref())?;
        }

        Ok(size)
    }

    fn write_payload(&self, to_stream: &mut dyn WriteAndSeek) -> Result<()> {
        write_jumbf(&self.desc, to_stream)?;

        for child in &self.child_boxes {
            write_jumbf(child.as_ref(), to_stream)?;
        }

        Ok(())
    }
}

/// This struct is used by `SuperBoxBuilder` to construct the description
/// box that is a required part of the superbox JUMBF data structure.
///
/// Since there must be exactly one description box per superbox, this
/// struct's API is not public. Instead, the APIs for setting fields in the
/// description box are available as part of `SuperBoxBuilder`. The description
/// box is generated automatically when `SuperBoxBuilder.write_jumbf()` is
/// called.
struct DescriptionBoxBuilder {
    /// Application-specific UUID for the superbox's data type.
    uuid: [u8; 16],

    /// Application-specific label for the superbox.
    label: Option<String>,

    /// True if the superbox containing this description box can
    /// be requested.
    requestable: bool,

    /// Application-specific 32-bit ID.
    id: Option<u32>,

    /// SHA-256 hash of the superbox's data payload.
    hash: Option<[u8; 32]>,

    /// Application-specific "private" box within description box.
    private: Option<Box<dyn ToBox>>,
}

impl DescriptionBoxBuilder {
    fn new(uuid: &[u8; 16]) -> Self {
        Self {
            uuid: *uuid,
            label: None,
            requestable: false,
            id: None,
            hash: None,
            private: None,
        }
    }
}

impl ToBox for DescriptionBoxBuilder {
    fn box_type(&self) -> BoxType {
        DESCRIPTION_BOX_TYPE
    }

    fn write_payload(&self, to_stream: &mut dyn WriteAndSeek) -> Result<()> {
        use crate::toggles;

        to_stream.write_all(&self.uuid)?;

        // Calculate toggles byte.
        let mut toggles = 0u8;

        // Toggle bit 0 (0x01) indicates that this superbox can be requested
        // via URI requests.
        if self.requestable {
            toggles |= toggles::REQUESTABLE;
        }

        // Toggle bit 1 (0x02) indicates that the label has an optional textual label.
        if self.label.is_some() {
            toggles |= toggles::HAS_LABEL;
        }

        // Toggle bit 2 (0x04) indicates that the label has an optional
        // application-specific 32-bit identifier.
        if self.id.is_some() {
            toggles |= toggles::HAS_ID;
        }

        // Toggle bit 3 (0x08) indicates that a SHA-256 hash of the superbox's
        // data box is present.
        if self.hash.is_some() {
            toggles |= toggles::HAS_HASH;
        }

        // Toggle bit 4 (0x10) indicates that an application-specific "private"
        // box is contained within the description box.
        if self.private.is_some() {
            toggles |= toggles::HAS_PRIVATE_BOX;
        }

        let toggles_slice = [toggles];
        to_stream.write_all(&toggles_slice)?;

        if let Some(label) = self.label.as_ref() {
            to_stream.write_all(label.as_bytes())?;
            to_stream.write_all(&[0u8])?;
        }

        if let Some(id) = self.id {
            write_be_u32(to_stream, id)?;
        }

        if let Some(hash) = self.hash {
            to_stream.write_all(&hash)?;
        }

        if let Some(private) = self.private.as_ref() {
            write_jumbf(private.as_ref(), to_stream)?;
        }

        Ok(())
    }
}

// DESIGN NOTE: This looks a lot like (and was inspired by) the built-in
// `Cow` type, but is distinct for a couple of reasons:
//
// 1. We're hosting `dyn ToBox` references or (owned) structs and I don't
//    believe that `ToOwned` can be implemented over `dyn (trait)`.
// 2. In this particular use case, we never need to convert between referenced
//    and owned data. This allows us to use this simpler implementation.
enum OwnedOrBorrowedBox<'a> {
    OwnedBox(Box<dyn ToBox>),
    BorrowedBox(&'a dyn ToBox),
}

impl<'a> OwnedOrBorrowedBox<'a> {
    fn as_ref(&self) -> &dyn ToBox {
        match self {
            OwnedOrBorrowedBox::OwnedBox(boxx) => boxx.as_ref(),
            OwnedOrBorrowedBox::BorrowedBox(boxx) => *boxx,
        }
    }
}

fn write_be_u32(to_stream: &mut dyn WriteAndSeek, v: u32) -> Result<()> {
    // Q&D implementation of big-endian formatting.
    let v_slice: [u8; 4] = [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8];
    to_stream.write_all(&v_slice)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use std::io::Cursor;

    use hex_literal::hex;

    use crate::{
        builder::{DataBoxBuilder, PlaceholderDataBox, SuperBoxBuilder},
        BoxType,
    };

    // Used here as an illustration only. This crate does not parse JSON content.
    const JSON_BOX_TYPE: BoxType = BoxType(*b"json");

    const RANDOM_BOX_TYPE: BoxType = BoxType(*b"abcd");

    #[test]
    fn basic_case() {
        let expected_jumbf = hex!(
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e64657363626f7800" // label
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .set_label("test.descbox");

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn non_requestable_label() {
        let expected_jumbf = hex!(
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "02" // toggles
                "746573742e64657363626f7800" // label
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .set_non_requestable_label("test.descbox");

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn with_id() {
        let expected_jumbf = hex!(
            "00000025" // box size
            "6a756d62" // box type = 'jumb'
                "0000001d" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "04" // toggles
                "00001000" // ID
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000")).set_id(4096);

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn with_hash() {
        let expected_jumbf = hex!(
            "0000004e" // box size
            "6a756d62" // box type = 'jumb'
                "00000046" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "0b" // toggles
                "746573742e64657363626f7800" // label
                "54686973206973206120626f67757320"
                "686173682e2e2e2e2e2e2e2e2e2e2e2e" // hash
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .set_label("test.descbox")
            .set_sha256_hash(b"This is a bogus hash............" as &[u8; 32]);

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn with_private_box() {
        let expected_jumbf = hex!(
            "00000057" // box size
            "6a756d62" // box type = 'jumb'
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

        let private = DataBoxBuilder::from_owned(
            JSON_BOX_TYPE,
            hex!("7b20226c6f636174696f6e223a20224d61726761"
                   "746520436974792c204e4a227d")
            .to_vec(),
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .set_label("test.descbox")
            .set_private_box(private);

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn no_label() {
        let expected_jumbf = hex!(
            "00000021" // box size
            "6a756d62" // box type = 'jumb'
                "00000019" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "00" // toggles
        );

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"));

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn with_child_boxes() {
        let expected_jumbf = hex!(
            "00000056" // box size
            "6a756d62" // box type = 'jumb'
                "00000019" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "00" // toggles
                // ---
                "00000029" // box size
                "6a736f6e" // box type = 'json'
                "7b20226c6f636174696f6e223a20224d61726761"
                "746520436974792c204e4a227d" // payload (JSON)
                // ---
                "0000000c" // box size
                "61626364" // box type = 'abcd'
                "41424344" // payload
        );

        let cbox1 = DataBoxBuilder::from_owned(
            JSON_BOX_TYPE,
            hex!("7b20226c6f636174696f6e223a20224d61726761"
                   "746520436974792c204e4a227d")
            .to_vec(),
        );

        let cbox2 = DataBoxBuilder::from_borrowed(RANDOM_BOX_TYPE, b"ABCD");

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .add_child_box(cbox1)
            .add_child_box(cbox2);

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.into_inner(), expected_jumbf);
    }

    #[test]
    fn with_placeholder() {
        let expected_jumbf = hex!(
            "00000062" // box size
            "6a756d62" // box type = 'jumb'
                "00000019" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "00" // toggles
                // ---
                "00000029" // box size
                "6a736f6e" // box type = 'json'
                "7b20226c6f636174696f6e223a20224d61726761"
                "746520436974792c204e4a227d" // payload (JSON)
                // ---
                "00000018" // box size
                "61626364" // box type = 'abcd'
                "00000000000000000000000000000000" // placeholder
        );

        let cbox = DataBoxBuilder::from_owned(
            JSON_BOX_TYPE,
            hex!("7b20226c6f636174696f6e223a20224d61726761"
                   "746520436974792c204e4a227d")
            .to_vec(),
        );

        let pbox = PlaceholderDataBox::new(RANDOM_BOX_TYPE, 16);

        let sbox = SuperBoxBuilder::new(&hex!("00000000000000000000000000000000"))
            .add_child_box(cbox)
            .add_borrowed_child_box(&pbox);

        let mut jumbf = Cursor::new(Vec::<u8>::new());
        sbox.write_jumbf(&mut jumbf).unwrap();
        assert_eq!(*jumbf.get_ref(), expected_jumbf);

        pbox.replace_payload(&mut jumbf, b"0123456789abcdef")
            .unwrap();

        let expected_jumbf = hex!(
            "00000062" // box size
            "6a756d62" // box type = 'jumb'
                "00000019" // box size
                "6a756d64" // box type = 'jumd'
                "00000000000000000000000000000000" // UUID
                "00" // toggles
                // ---
                "00000029" // box size
                "6a736f6e" // box type = 'json'
                "7b20226c6f636174696f6e223a20224d61726761"
                "746520436974792c204e4a227d" // payload (JSON)
                // ---
                "00000018" // box size
                "61626364" // box type = 'abcd'
                "30313233343536373839616263646566" // replaced payload
        );

        assert_eq!(*jumbf.get_ref(), expected_jumbf);
    }
}
