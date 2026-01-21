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

use std::io::{Error, Result, Seek, SeekFrom, Write};

use crate::BoxType;

const MAX_32BIT_PAYLOAD_SIZE: usize = 0xfffffff7;

/// The `ToBox` trait allows any data type to generate a JUMBF data box.
///
/// ## Design constraints
///
/// Any stream presented here must implement the standard [`Write`] and [`Seek`]
/// traits. While [`Seek`] is not strictly necessary for all use cases, it was
/// not possible to implement the [`PlaceholderDataBox`] type without requiring
/// [`Seek`].
///
/// [`box_type()`]: Self::box_type()
/// [`write_payload()`]: Self::write_payload()
/// [`PlaceholderDataBox`]: crate::builder::PlaceholderDataBox
pub trait ToBox {
    /// Specifies the type of information which will be provided by the
    /// [`write_payload()`] method.
    ///
    /// The value of this field is encoded as a 4-byte big-endian
    /// unsigned integer. However, boxes are generally referred to by an
    /// ISO/IEC 646 character string translation of the integer value.
    ///
    /// For that reason, this is represented here as a 4-byte slice.
    ///
    /// The box type can typically be provided with a byte string constant (i.e.
    /// `b"jumd"`).
    ///
    /// [`write_payload()`]: Self::write_payload()
    fn box_type(&self) -> BoxType;

    /// Returns the size of the payload which will be provided by the
    /// [`write_payload()`] method.
    ///
    /// If the size of the payload is known ahead of time (as, for example,
    /// in [`DataBoxBuilder`] where the payload is a byte buffer that is already
    /// available), this function should be re-implemented to return that
    /// size. This will improve the performance of subsequent JUMBF
    /// generation.
    ///
    /// The default implementation here will invoke
    /// [`write_payload()`] an extra time during box size calculation and write
    /// the content into a throwaway sink stream. Depending on the complexity
    /// of [`write_payload()`], this could be expensive.
    ///
    /// [`DataBoxBuilder`]: crate::builder::DataBoxBuilder
    /// [`write_payload()`]: Self::write_payload()
    fn payload_size(&self) -> Result<usize> {
        let mut counting_sink = CountingSink::default();
        self.write_payload(&mut counting_sink)?;
        counting_sink.flush()?;
        Ok(counting_sink.count)
    }

    /// Write the payload for this box to the JUMBF stream.
    ///
    /// The payload must be exactly the size previously specified by
    /// [`payload_size()`].
    ///
    /// [`payload_size()`]: Self::payload_size()
    fn write_payload(&self, to_stream: &mut dyn WriteAndSeek) -> Result<()>;
}

pub(crate) fn jumbf_size(boxx: &dyn ToBox) -> Result<usize> {
    Ok(jumbf_size_from_payload_size(boxx.payload_size()?))
}

pub(crate) fn write_jumbf(boxx: &dyn ToBox, to_stream: &mut dyn WriteAndSeek) -> Result<()> {
    let payload_size = boxx.payload_size()?;
    let jumbf_size = jumbf_size_from_payload_size(payload_size);

    if payload_size <= MAX_32BIT_PAYLOAD_SIZE {
        let size_slice: [u8; 4] = [
            (jumbf_size >> 24) as u8,
            (jumbf_size >> 16) as u8,
            (jumbf_size >> 8) as u8,
            jumbf_size as u8,
        ];
        to_stream.write_all(&size_slice)?;
    } else {
        // TO DO: Support for >4GB payloads.
        unimplemented!();
    }

    // TO DO: Check stream position and verify that exactly the
    // specified number of bytes was written.

    let box_type = boxx.box_type();
    to_stream.write_all(&box_type.0)?;

    boxx.write_payload(to_stream)?;

    Ok(())
}

fn jumbf_size_from_payload_size(payload_size: usize) -> usize {
    if payload_size <= MAX_32BIT_PAYLOAD_SIZE {
        payload_size + 8
    } else {
        // TO DO: Support for >4GB payloads.
        unimplemented!();
        // payload_size + 16
    }
}

/// A stream that implements [`Write`] and [`Seek`] traits.
///
/// Required for [`ToBox`].
pub trait WriteAndSeek: Write + Seek {}
impl<T: Write + Seek> WriteAndSeek for T {}

#[derive(Default)]
struct CountingSink {
    count: usize,
}

impl Seek for CountingSink {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        // Shouldn't need to seek while counting payload size.
        Err(Error::other(
            "shouldn't need to seek while calculating payload size",
        ))
    }
}

impl Write for CountingSink {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let len = buf.len();
        self.count += len;
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
