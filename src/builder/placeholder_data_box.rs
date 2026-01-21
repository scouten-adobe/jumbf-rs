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
    io::{Error, Result},
};

use crate::{
    builder::{ToBox, WriteAndSeek},
    BoxType,
};

/// A `PlaceholderDataBox` allows you to reserve space in a JUMBF data structure
/// for content that will be filled in after the overall JUMBF data structure is
/// created.
///
/// You can specify a data size to reserve. When the initial JUMBF data
/// structure is created, the box will be zero-filled to the specified size.
/// Later, you can call [`replace_payload()`] to replace that reserved space
/// with new content.
///
/// [`replace_payload()`]: Self::replace_payload()
pub struct PlaceholderDataBox {
    tbox: BoxType,
    size: usize,
    offset: RefCell<Option<u64>>,
}

impl PlaceholderDataBox {
    /// Create a new placeholder data box that will reserve `size` bytes.
    ///
    /// The box will be given the JUMBF box type specified by `tbox`.
    pub fn new(tbox: BoxType, size: usize) -> Self {
        Self {
            tbox,
            size,
            offset: RefCell::new(None),
        }
    }

    /// Return the offset in the stream where the payload can be written.
    ///
    /// Will return `None` before the superbox's [`write_jumbf()`] method is
    /// called.
    ///
    /// [`write_jumbf()`]: crate::builder::SuperBoxBuilder::write_jumbf()
    pub fn offset(&self) -> Option<u64> {
        self.offset.clone().into_inner()
    }

    /// Replace the zero-filled placeholder content with actual content.
    ///
    /// An error will be returned if `payload` is larger than the placeholder
    /// size specified when this `PlaceholderDataBox` was created.
    ///
    /// Assuming the placeholder is not larger than the initial reservation,
    /// this method will seek the stream to [`offset()`] and write the new
    /// payload at that location.
    ///
    /// [`offset()`]: Self::offset()
    pub fn replace_payload(&self, to_stream: &mut dyn WriteAndSeek, payload: &[u8]) -> Result<()> {
        if payload.len() > self.size {
            return Err(Error::other(
                format!("replace_payload: payload ({len} bytes) is larger than reserved capacity ({reserve} bytes)", len = payload.len(), reserve = self.size)
            ));
        }

        let offset = self.offset.borrow();

        if let Some(offset) = *offset {
            to_stream.seek(std::io::SeekFrom::Start(offset))?;
            to_stream.write_all(payload)
        } else {
            // HINT: If you receive this error, be sure to call write_jumbf() on a superbox
            // containing this box first.

            Err(Error::other(
                "replace_payload: no offset recorded; call write_jumbf() first".to_string(),
            ))
        }
    }
}

impl ToBox for PlaceholderDataBox {
    fn box_type(&self) -> BoxType {
        self.tbox
    }

    fn payload_size(&self) -> Result<usize> {
        Ok(self.size)
    }

    fn write_payload(&self, to_stream: &mut dyn WriteAndSeek) -> Result<()> {
        let offset = to_stream.stream_position()?;

        match offset {
            0 => {
                return Err(Error::other(
                    "placeholder stream should have some data already",
                ));
            }
            _ => {
                self.offset.replace(Some(offset));
            }
        };

        let zeros: Vec<u8> = vec![0; self.size];
        to_stream.write_all(&zeros)?;
        Ok(())
    }
}
