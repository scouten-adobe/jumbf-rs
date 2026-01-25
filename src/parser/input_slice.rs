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
    io::{Read, Seek, SeekFrom},
    rc::Rc,
};

/// A lazy reference to data within a readable/seekable source.
///
/// This type represents data by its position and length rather than
/// holding the actual bytes. Data is only read when explicitly requested
/// via [`to_vec()`](Self::to_vec) or [`read_range()`](Self::read_range).
///
/// This allows efficient parsing where only metadata is read initially,
/// and payload data is only loaded if/when the client needs it.
///
/// # Example
///
/// ```
/// use std::io::Cursor;
/// use std::rc::Rc;
/// use std::cell::RefCell;
/// use jumbf::parser::InputSlice;
///
/// let data = vec![0u8, 1, 2, 3, 4, 5];
/// let reader = Rc::new(RefCell::new(Cursor::new(data)));
///
/// let slice = InputSlice {
///     reader: Rc::clone(&reader),
///     offset: 2,
///     len: 3,
/// };
///
/// assert_eq!(slice.len(), 3);
/// assert_eq!(slice.to_vec().unwrap(), vec![2, 3, 4]);
/// ```
pub struct InputSlice<R> {
    /// Shared reference to the reader.
    pub(crate) reader: Rc<RefCell<R>>,

    /// Position of the data within the reader.
    pub(crate) offset: u64,

    /// Length of the data in bytes.
    pub(crate) len: usize,
}

impl<R: Read + Seek> InputSlice<R> {
    /// Create a new InputSlice.
    pub fn new(reader: Rc<RefCell<R>>, offset: u64, len: usize) -> Self {
        Self {
            reader,
            offset,
            len,
        }
    }

    /// Read the entire data into a Vec.
    ///
    /// This performs I/O to read the bytes from the underlying reader.
    pub fn to_vec(&self) -> std::io::Result<Vec<u8>> {
        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(self.offset))?;
        let mut buf = vec![0u8; self.len];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read a portion of the data.
    ///
    /// # Arguments
    ///
    /// * `start` - Offset within this slice (not the reader).
    /// * `len` - Number of bytes to read.
    ///
    /// # Errors
    ///
    /// Returns an error if the range exceeds the slice bounds or if I/O fails.
    pub fn read_range(&self, start: usize, len: usize) -> std::io::Result<Vec<u8>> {
        if start + len > self.len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Range exceeds slice bounds",
            ));
        }

        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(self.offset + start as u64))?;
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Get the length of the data without reading it.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if this slice is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the offset within the source reader.
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl<R> std::fmt::Debug for InputSlice<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputSlice")
            .field("offset", &self.offset)
            .field("len", &self.len)
            .finish()
    }
}

impl<R> Clone for InputSlice<R> {
    fn clone(&self) -> Self {
        Self {
            reader: Rc::clone(&self.reader),
            offset: self.offset,
            len: self.len,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::io::Cursor;

    #[test]
    fn to_vec() {
        let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));

        let slice = InputSlice::new(Rc::clone(&reader), 2, 5);
        assert_eq!(slice.to_vec().unwrap(), vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn read_range() {
        let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));

        let slice = InputSlice::new(Rc::clone(&reader), 2, 5);
        assert_eq!(slice.read_range(1, 3).unwrap(), vec![3, 4, 5]);
    }

    #[test]
    fn read_range_out_of_bounds() {
        let data = vec![0u8, 1, 2, 3, 4, 5];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));

        let slice = InputSlice::new(Rc::clone(&reader), 2, 3);
        assert!(slice.read_range(2, 3).is_err());
    }

    #[test]
    fn len_and_offset() {
        let data = vec![0u8; 10];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));

        let slice = InputSlice::new(reader, 5, 3);
        assert_eq!(slice.len(), 3);
        assert_eq!(slice.offset(), 5);
        assert!(!slice.is_empty());
    }

    #[test]
    fn empty_slice() {
        let data = vec![0u8; 10];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));

        let slice = InputSlice::new(reader, 5, 0);
        assert_eq!(slice.len(), 0);
        assert!(slice.is_empty());
        assert_eq!(slice.to_vec().unwrap(), Vec::<u8>::new());
    }
}
