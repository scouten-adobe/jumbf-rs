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

/// Marker type used as the reader type parameter for slice-based parsing.
///
/// This type will never implement `Read` or `Seek`, ensuring that there is
/// no overlap between slice-based and reader-based implementations.
///
/// Users don't typically need to reference this type directly; it's used
/// as the default parameter for parser types like `DataBox<'a, R = NoReader>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoReader;
