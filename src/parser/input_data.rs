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

use std::io::{Read, Seek};

use crate::parser::{InputSlice, NoReader};

/// Data that can be either borrowed from memory or lazily loaded from a reader.
///
/// This enum allows the parser to work with both in-memory slices (zero-copy)
/// and streaming readers (lazy-loaded). The data is only read when explicitly
/// requested via [`to_vec()`](Self::to_vec).
///
/// # Examples
///
/// ## Borrowed data (zero-copy)
///
/// ```
/// use jumbf::parser::InputData;
///
/// let data = vec![1, 2, 3, 4, 5];
/// let input = InputData::Borrowed(&data[1..4]);
///
/// assert_eq!(input.len(), 3);
/// assert_eq!(input.as_slice().unwrap(), &[2, 3, 4]);
/// assert_eq!(input.to_vec().unwrap(), vec![2, 3, 4]);
/// ```
///
/// ## Lazy data (from reader)
///
/// ```
/// use std::io::Cursor;
/// use std::rc::Rc;
/// use std::cell::RefCell;
/// use jumbf::parser::{InputData, InputSlice};
///
/// let data = vec![1, 2, 3, 4, 5];
/// let reader = Rc::new(RefCell::new(Cursor::new(data)));
/// let slice = InputSlice::new(reader, 1, 3);
/// let input = InputData::Lazy(slice);
///
/// assert_eq!(input.len(), 3);
/// assert!(input.as_slice().is_none()); // Can't get slice from lazy data
/// assert_eq!(input.to_vec().unwrap(), vec![2, 3, 4]); // Reads from reader
/// ```
pub enum InputData<'a, R> {
    /// Data borrowed from an in-memory slice (zero-copy).
    Borrowed(&'a [u8]),

    /// Data referenced by position in a reader (lazy-loaded).
    Lazy(InputSlice<R>),
}

impl<'a, R> InputData<'a, R> {
    /// Get the length without reading data.
    pub fn len(&self) -> usize {
        match self {
            Self::Borrowed(slice) => slice.len(),
            Self::Lazy(input_slice) => input_slice.len,
        }
    }

    /// Check if this data is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the data as a borrowed slice if available.
    ///
    /// Returns `Some(&[u8])` for borrowed data, `None` for lazy data.
    pub fn as_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Borrowed(slice) => Some(slice),
            Self::Lazy(_) => None,
        }
    }

    /// Check if this is borrowed data.
    pub fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(_))
    }

    /// Check if this is lazy data.
    pub fn is_lazy(&self) -> bool {
        matches!(self, Self::Lazy(_))
    }
}

impl<'a, R: Read + Seek> InputData<'a, R> {
    /// Read the data into a Vec.
    ///
    /// For borrowed data, this clones the slice.
    /// For lazy data, this reads from the reader.
    pub fn to_vec(&self) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Borrowed(slice) => Ok(slice.to_vec()),
            Self::Lazy(input_slice) => input_slice.to_vec(),
        }
    }
}

impl<'a> InputData<'a, NoReader> {
    /// Read the data into a Vec (for borrowed data with NoReader).
    ///
    /// This is a convenience method for slice-based parsing where
    /// there is no actual reader.
    pub fn to_vec(&self) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Borrowed(slice) => Ok(slice.to_vec()),
            Self::Lazy(_) => unreachable!("Lazy data cannot exist with NoReader"),
        }
    }
}

impl<'a, R> std::fmt::Debug for InputData<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Borrowed(slice) => {
                let preview_len = slice.len().min(20);
                let preview: Vec<String> = slice[..preview_len]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                
                write!(
                    f,
                    "InputData::Borrowed({} bytes: [{}{}])",
                    slice.len(),
                    preview.join(", "),
                    if slice.len() > 20 { ", ..." } else { "" }
                )
            }
            Self::Lazy(input_slice) => write!(f, "InputData::Lazy({:?})", input_slice),
        }
    }
}

impl<'a, R> PartialEq for InputData<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(a), Self::Borrowed(b)) => a == b,
            (Self::Lazy(a), Self::Lazy(b)) => {
                a.offset == b.offset && a.len == b.len
            }
            _ => false,
        }
    }
}

impl<'a, R> Eq for InputData<'a, R> {}

impl<'a, R> Clone for InputData<'a, R> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(slice) => Self::Borrowed(slice),
            Self::Lazy(input_slice) => Self::Lazy((*input_slice).clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::{cell::RefCell, io::Cursor, rc::Rc};

    #[test]
    fn borrowed_data() {
        let data = vec![1, 2, 3, 4, 5];
        let input = InputData::<NoReader>::Borrowed(&data[1..4]);

        assert_eq!(input.len(), 3);
        assert!(!input.is_empty());
        assert!(input.is_borrowed());
        assert!(!input.is_lazy());
        assert_eq!(input.as_slice().unwrap(), &[2, 3, 4]);
        assert_eq!(input.to_vec().unwrap(), vec![2, 3, 4]);
    }

    #[test]
    fn lazy_data() {
        let data = vec![1, 2, 3, 4, 5];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));
        let slice = InputSlice::new(reader, 1, 3);
        let input = InputData::Lazy(slice);

        assert_eq!(input.len(), 3);
        assert!(!input.is_empty());
        assert!(!input.is_borrowed());
        assert!(input.is_lazy());
        assert!(input.as_slice().is_none());
        assert_eq!(input.to_vec().unwrap(), vec![2, 3, 4]);
    }

    #[test]
    fn empty_borrowed() {
        let data: Vec<u8> = vec![];
        let input = InputData::<NoReader>::Borrowed(&data);

        assert_eq!(input.len(), 0);
        assert!(input.is_empty());
        assert_eq!(input.to_vec().unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn empty_lazy() {
        let data = vec![1, 2, 3];
        let reader = Rc::new(RefCell::new(Cursor::new(data)));
        let slice = InputSlice::new(reader, 1, 0);
        let input = InputData::Lazy(slice);

        assert_eq!(input.len(), 0);
        assert!(input.is_empty());
        assert_eq!(input.to_vec().unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn equality() {
        let data1 = vec![1, 2, 3];
        let data2 = vec![1, 2, 3];

        let b1 = InputData::<NoReader>::Borrowed(&data1);
        let b2 = InputData::<NoReader>::Borrowed(&data2);
        assert_eq!(b1, b2);

        let reader1 = Rc::new(RefCell::new(Cursor::new(vec![1, 2, 3])));
        let reader2 = Rc::new(RefCell::new(Cursor::new(vec![1, 2, 3])));
        let l1 = InputData::Lazy(InputSlice::new(reader1, 0, 3));
        let l2 = InputData::Lazy(InputSlice::new(reader2, 0, 3));
        assert_eq!(l1, l2);

        // Borrowed and lazy are never equal
        let b = InputData::<Cursor<Vec<u8>>>::Borrowed(&data1);
        assert_ne!(b, l1);
    }
}
