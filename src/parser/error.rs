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

use std::str::Utf8Error;

use crate::BoxType;

/// The error type for JUMBF parsing operations.
#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// Invalid length value.
    #[error("Box length value {0} is reserved")]
    InvalidBoxLength(u32),

    /// Not a description box.
    #[error("Superbox box type should be 'jumb', was '{0:#?}'")]
    InvalidSuperBoxType(BoxType),

    /// Not a description box.
    #[error("Description box type should be 'jumd', was '{0:#?}'")]
    InvalidDescriptionBoxType(BoxType),

    /// UTF-8 decoding error.
    #[error("Unable to decode description box as UTF-8: {0:?}")]
    Utf8Error(Utf8Error),

    /// JUMBF data was incomplete.
    #[error("Incomplete data, needed {0} more bytes")]
    Incomplete(usize),
}

/// Holds the result of JUMBF parsing functions.
///
/// Note that this type is also a [`Result`], so the usual functions (`map`,
/// `unwrap`, etc.) are available.
pub type ParseResult<'a, T, E = crate::parser::Error> = Result<(&'a [u8], T), E>;
