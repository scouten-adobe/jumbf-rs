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

use hex_literal::hex;
use pretty_assertions_sorted::assert_eq;

use crate::{
    parser::{DataBox, DescriptionBox, Error},
    BoxType,
};

#[test]
fn from_slice() {
    let jumbf = hex!(
        "00000026" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "03" // toggles
        "746573742e64657363626f7800" // label
    );

    let (rem, dbox) = DescriptionBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: Some("test.descbox",),
            requestable: true,
            id: None,
            hash: None,
            private: None,
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: None,\n    original: 38 bytes starting with [00, 00, 00, 26, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
}

#[test]
fn from_box() {
    let jumbf = hex!(
        "00000026" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "03" // toggles
        "746573742e64657363626f7800" // label
    );

    let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: Some("test.descbox",),
            requestable: true,
            id: None,
            hash: None,
            private: None,
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: None,\n    original: 38 bytes starting with [00, 00, 00, 26, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
}

#[test]
fn with_id() {
    let jumbf = hex!(
        "0000001d" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "04" // toggles
        "00001000" // ID
    );

    let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: None,
            requestable: false,
            id: Some(4096),
            hash: None,
            private: None,
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: None,\n    requestable: false,\n    id: Some(\n        4096,\n    ),\n    hash: None,\n    private: None,\n    original: 29 bytes starting with [00, 00, 00, 1d, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
}

#[test]
fn error_incomplete_id() {
    let jumbf = hex!(
        "0000001c" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "04" // toggles
        "000010" // ID (incomplete)
    );

    assert_eq!(
        DescriptionBox::from_slice(&jumbf).unwrap_err(),
        Error::Incomplete(1)
    );
}

#[test]
fn with_hash() {
    let jumbf = hex!(
        "00000046" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "0b" // toggles
        "746573742e64657363626f7800" // label
        "54686973206973206120626f67757320"
        "686173682e2e2e2e2e2e2e2e2e2e2e2e" // hash
    );

    let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: Some("test.descbox",),
            requestable: true,
            id: None,
            hash: Some(b"This is a bogus hash............" as &[u8; 32]),
            private: None,
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: Some(32 bytes starting with [54, 68, 69, 73, 20, 69, 73, 20, 61, 20, 62, 6f, 67, 75, 73, 20, 68, 61, 73, 68]),\n    private: None,\n    original: 70 bytes starting with [00, 00, 00, 46, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
}

#[test]
fn with_private_box() {
    let jumbf = hex!(
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

    let (rem, boxx) = DataBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    let (rem, dbox) = DescriptionBox::from_box(boxx).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: Some("test.descbox",),
            requestable: true,
            id: None,
            hash: None,
            private: Some(DataBox {
                tbox: BoxType(*b"json"),
                data: &[
                    123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58, 32, 34, 77, 97, 114,
                    103, 97, 116, 101, 32, 67, 105, 116, 121, 44, 32, 78, 74, 34, 125,
                ],
                original: &jumbf[38..79],
            }),
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: Some(\n        \"test.descbox\",\n    ),\n    requestable: true,\n    id: None,\n    hash: None,\n    private: Some(\n        DataBox {\n            tbox: b\"json\",\n            data: 33 bytes starting with [7b, 20, 22, 6c, 6f, 63, 61, 74, 69, 6f, 6e, 22, 3a, 20, 22, 4d, 61, 72, 67, 61],\n            original: 41 bytes starting with [00, 00, 00, 29, 6a, 73, 6f, 6e, 7b, 20, 22, 6c, 6f, 63, 61, 74, 69, 6f, 6e, 22],\n        },\n    ),\n    original: 79 bytes starting with [00, 00, 00, 4f, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
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
        DescriptionBox::from_slice(&jumbf).unwrap_err(),
        Error::InvalidDescriptionBoxType(BoxType(*b"jumc"))
    );
}

#[test]
fn error_incomplete_uuid() {
    let jumbf = hex!(
        "00000016" // box size
        "6a756d64" // box type = 'jumd'
        "0000000000000000000000000000" // UUID (incomplete)
    );

    assert_eq!(
        DescriptionBox::from_slice(&jumbf).unwrap_err(),
        Error::Incomplete(2)
    );
}

#[test]
fn no_label() {
    let jumbf = hex!(
        "00000019" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "00" // toggles
    );

    let (rem, dbox) = DescriptionBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        dbox,
        DescriptionBox {
            uuid: &[0; 16],
            label: None,
            requestable: false,
            id: None,
            hash: None,
            private: None,
            original: &jumbf,
        }
    );

    assert_eq!(format!("{dbox:#?}"), "DescriptionBox {\n    uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    label: None,\n    requestable: false,\n    id: None,\n    hash: None,\n    private: None,\n    original: 25 bytes starting with [00, 00, 00, 19, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n}");
}

#[test]
fn error_incomplete_hash() {
    let jumbf = hex!(
        "00000044" // box size
        "6a756d64" // box type = 'jumd'
        "00000000000000000000000000000000" // UUID
        "0b" // toggles
        "746573742e64657363626f7800" // label
        "54686973206973206120626f67757320"
        "686173682e2e2e2e2e2e2e2e2e2e" // hash (incomplete)
    );

    assert_eq!(
        DescriptionBox::from_slice(&jumbf).unwrap_err(),
        Error::Incomplete(2)
    );
}
