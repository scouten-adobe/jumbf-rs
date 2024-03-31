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

use std::fmt::{Debug, Formatter};

use crate::{
    box_type::SUPER_BOX_TYPE,
    debug::*,
    parser::{DataBox, DescriptionBox, Error, Source},
};

/// A JUMBF superbox contains a description box and zero or more
/// data boxes, each of which may or may not be a superbox.
#[derive(Clone, Eq, PartialEq)]
pub struct SuperBox<S: Source> {
    /// Description box.
    pub desc: DescriptionBox<S>,

    /// Child boxes. (These are referred to in some documentation
    /// as "data boxes.")
    pub child_boxes: Vec<ChildBox<S>>,

    /// Original box data.
    ///
    /// This the original byte slice that was parsed to create this box.
    /// It is preserved in case a future client wishes to re-serialize this
    /// box as is.
    pub original: S,
}

impl<S: Source> SuperBox<S> {
    /// Parse a source as a JUMBF superbox, and return a tuple of the
    /// remainder of the input and the parsed super box. Children of this
    /// superbox which are also superboxes will be parsed recursively without
    /// limit.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_source(original: S) -> Result<(Self, S), Error<S::Error>> {
        Self::from_source_with_depth_limit(original, usize::MAX)
    }

    /// Parse a byte-slice as a JUMBF superbox, and return a tuple of the
    /// remainder of the input and the parsed super box. Children of this
    /// superbox which are also superboxes will be parsed recursively, to a
    /// limit of `depth_limit` nested boxes.
    ///
    /// If `depth_limit` is 0, any child superboxes that are found will be
    /// returned as plain [`DataBox`] structs instead.
    ///
    /// The returned object uses zero-copy, and so has the same lifetime as the
    /// input.
    pub fn from_source_with_depth_limit(
        original: S,
        depth_limit: usize,
    ) -> Result<(Self, S), Error<S::Error>> {
        let (data_box, rem) = DataBox::from_source(original)?;
        Ok((
            Self::from_data_box_with_depth_limit(&data_box, depth_limit)?,
            rem,
        ))
    }

    /// Re-parse a [`DataBox`] as a JUMBF superbox. Children of this
    /// superbox which are also superboxes will be parsed recursively without
    /// limit.
    ///
    /// If the box is of `jumb` type and has the correct structure, returns
    /// a tuple of the remainder of the input from the box (which should
    /// typically be empty) and the new [`SuperBox`] object.
    ///
    /// Will return an error if the box isn't of `jumb` type.
    pub fn from_data_box(data_box: &DataBox<S>) -> Result<Self, Error<S::Error>> {
        Self::from_data_box_with_depth_limit(data_box, usize::MAX)
    }

    /// Re-parse a [`DataBox`] as a JUMBF superbox. Children of this superbox
    /// which are also superboxes will be parsed recursively, to a limit of
    /// `depth_limit` nested boxes.
    ///
    /// If the box is of `jumb` type and has the correct structure, returns
    /// a tuple of the remainder of the input from the box (which should
    /// typically be empty) and the new [`SuperBox`] object. If `depth_limit` is
    /// 0, any child superboxes that are found will be returned as plain
    /// [`DataBox`] structs instead.
    ///
    /// Will return an error if the box isn't of `jumb` type.
    pub fn from_data_box_with_depth_limit(
        data_box: &DataBox<S>,
        depth_limit: usize,
    ) -> Result<Self, Error<S::Error>> {
        if data_box.tbox != SUPER_BOX_TYPE {
            return Err(Error::InvalidSuperBoxType(data_box.tbox));
        }

        let (i, _) = data_box.data.split_at(data_box.data.len())?;
        let (desc, i) = DescriptionBox::from_source(i)?;

        let (child_boxes, _) = boxes_from_source(i)?;
        let child_boxes = child_boxes
            .into_iter()
            .map(|d| {
                if d.tbox == SUPER_BOX_TYPE && depth_limit > 0 {
                    let sbox = Self::from_data_box_with_depth_limit(&d, depth_limit - 1)?;
                    Ok(ChildBox::SuperBox(sbox))
                } else {
                    Ok(ChildBox::DataBox(d))
                }
            })
            .collect::<Result<Vec<ChildBox<S>>, Error<S::Error>>>()?;

        let (original, _) = data_box.original.split_at(data_box.original.len())?;
        Ok(Self {
            desc,
            child_boxes,
            original,
        })
    }

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

        let matching_children: Vec<&SuperBox<S>> = self
            .child_boxes
            .iter()
            .filter_map(|child_box| match child_box {
                ChildBox::SuperBox(sbox) => {
                    if let Some(sbox_label) = sbox.desc.label.as_ref() {
                        if sbox_label == label && sbox.desc.requestable {
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
                    return sbox.find_by_label(suffix);
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
    pub fn data_box(&self) -> Option<&DataBox<S>> {
        self.child_boxes
            .first()
            .and_then(|child_box| match child_box {
                ChildBox::DataBox(data_box) => Some(data_box),
                _ => None,
            })
    }
}

impl<S: Source + Debug> Debug for SuperBox<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("SuperBox")
            .field("desc", &self.desc)
            .field("child_boxes", &self.child_boxes)
            .field(
                "original",
                &DebugByteSlice(&self.original.as_bytes().unwrap_or_default()),
            )
            .finish()
    }
}

// Parse boxes from slice until source is empty.
fn boxes_from_source<S: Source>(i: S) -> Result<(Vec<DataBox<S>>, S), Error<S::Error>> {
    let mut result: Vec<DataBox<S>> = vec![];
    let mut i = i;

    while i.len() > 0 {
        let (dbox, x) = DataBox::from_source(i)?;
        i = x;
        result.push(dbox);
    }

    Ok((result, i))
}

/// This type represents a single box within a superbox,
/// which may itself be a superbox or or a regular box.
///
/// Note that this crate doesn't parse the content or ascribe
/// meaning to any type of box other than superbox (`jumb`) or
/// description box (`jumd`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildBox<S: Source> {
    /// A superbox.
    SuperBox(SuperBox<S>),

    /// Any other kind of box.
    DataBox(DataBox<S>),
}

impl<S: Source> ChildBox<S> {
    /// If this represents a nested super box, return a reference to that
    /// superbox.
    pub fn as_super_box(&self) -> Option<&SuperBox<S>> {
        if let Self::SuperBox(sb) = self {
            Some(sb)
        } else {
            None
        }
    }

    /// If this represents a nested data box, return a reference to that data
    /// box.
    pub fn as_data_box(&self) -> Option<&DataBox<S>> {
        if let Self::DataBox(db) = self {
            Some(db)
        } else {
            None
        }
    }
}
