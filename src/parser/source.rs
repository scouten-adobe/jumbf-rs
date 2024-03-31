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
    error::Error,
    fmt::{Debug, Display, Formatter},
};

pub trait Source: Debug + Sized {
    type Error: Debug;

    fn read_bytes(&self, data: &mut [u8]) -> Result<Self, Self::Error>;
    fn as_bytes(&self) -> Result<Vec<u8>, Self::Error>;
    fn len(&self) -> usize;

    fn split_at(&self, len: usize) -> Result<(Self, Self), Self::Error>;
    fn offset_of_subsource(&self, subsource: &Self) -> Option<usize>;

    fn read_u8(&self) -> Result<(u8, Self), Self::Error>;

    fn read_be32(&self) -> Result<(u32, Self), Self::Error> {
        let (be32, remainder) = self.split_at(4)?;

        let mut res = 0u32;
        let mut i = be32;

        loop {
            if let Ok((byte, x)) = i.read_u8() {
                i = x;
                res = (res << 8) + byte as u32;
            } else {
                break;
            }
        }

        Ok((res, remainder))
    }

    fn read_be64(&self) -> Result<(u64, Self), Self::Error> {
        let mut b = [0u8; 8];
        let remainder = self.read_bytes(&mut b)?;

        let mut res = 0u64;
        for byte in b {
            res = (res << 8) + byte as u64;
        }

        Ok((res, remainder))
    }
}

/// Returned when trying to read past the end of a slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadPastEndOfSlice {
    pub wanted: usize,
    pub have: usize,
}

impl Display for ReadPastEndOfSlice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Read past end of slice (wanted {wanted} bytes, have {have} bytes)",
            wanted = self.wanted,
            have = self.have
        )
    }
}

impl Error for ReadPastEndOfSlice {}

impl Source for &[u8] {
    type Error = ReadPastEndOfSlice;

    fn read_bytes(&self, data: &mut [u8]) -> Result<Self, Self::Error> {
        if data.len() > self.len() {
            return Err(ReadPastEndOfSlice {
                wanted: data.len(),
                have: self.len(),
            });
        }

        let self_as_u8: &[u8] = self;
        let (wanted, remainder) = self_as_u8.split_at(data.len());
        data.copy_from_slice(wanted);
        Ok(remainder)
    }

    fn as_bytes(&self) -> Result<Vec<u8>, Self::Error> {
        // Replace with COW
        Ok(self.to_vec())
    }

    fn len(&self) -> usize {
        let self_as_u8: &[u8] = self;
        self_as_u8.len()
    }

    fn read_u8(&self) -> Result<(u8, Self), Self::Error> {
        match self.split_first() {
            Some((byte, rem)) => Ok((*byte, rem)),
            None => Err(ReadPastEndOfSlice { wanted: 1, have: 0 }),
        }
    }

    fn split_at(&self, len: usize) -> Result<(Self, Self), Self::Error> {
        if len > self.len() {
            return Err(ReadPastEndOfSlice {
                wanted: len,
                have: self.len(),
            });
        }

        let self_as_u8: &[u8] = self;
        let (wanted, remainder) = self_as_u8.split_at(len);
        Ok((wanted, remainder))
    }

    fn offset_of_subsource(&self, subsource: &Self) -> Option<usize> {
        let self_as_ptr = self.as_ptr() as usize;
        let sub_as_ptr = subsource.as_ptr() as usize;

        if sub_as_ptr < self_as_ptr {
            return None;
        }

        let offset = sub_as_ptr.wrapping_sub(self_as_ptr);
        if offset + subsource.len() > self.len() {
            None
        } else {
            Some(offset)
        }
    }
}
