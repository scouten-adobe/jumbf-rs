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
    parser::{ChildBox, DataBox, DescriptionBox, Error, SuperBox},
    BoxType,
};

#[test]
fn simple_super_box() {
    let jumbf = hex!(
        "0000002f" // box size
        "6a756d62" // box type = 'jumb'
            "00000027" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e7375706572626f7800" // label
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.superbox"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..47],
            },
            child_boxes: vec!(),
            original: &jumbf,
        }
    );

    assert_eq!(format!("{sbox:#?}"), "SuperBox {\n    desc: DescriptionBox {\n        uuid: [00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n        label: Some(\n            \"test.superbox\",\n        ),\n        requestable: true,\n        id: None,\n        hash: None,\n        private: None,\n        original: 39 bytes starting with [00, 00, 00, 27, 6a, 75, 6d, 64, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00, 00],\n    },\n    child_boxes: [],\n    original: 47 bytes starting with [00, 00, 00, 2f, 6a, 75, 6d, 62, 00, 00, 00, 27, 6a, 75, 6d, 64, 00, 00, 00, 00],\n}");
}

#[test]
fn nested_super_boxes() {
    let jumbf = hex!(
        "00000065" // box size
        "6a756d62" // box type = 'jumb'
            "0000002f" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e7375706572626f785f64617461626f7800" // label
            // ------
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e64617461626f7800"
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.superbox_databox"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..55],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: &[0; 16],
                    label: Some("test.databox"),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &jumbf[63..101],
                },
                child_boxes: vec!(),
                original: &jumbf[55..101],
            })),
            original: &jumbf,
        }
    );
}

#[test]
fn child_superbox_without_label() {
    let jumbf = hex!(
        "00000058" // box size
        "6a756d62" // box type = 'jumb'
            "0000002f" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e7375706572626f785f64617461626f7800" // label
            // ------
            "00000021" // box size
            "6a756d62" // box type = 'jumb'
                "00000019" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "00" // toggles
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.superbox_databox"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..55],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: &[0; 16],
                    label: None,
                    requestable: false,
                    id: None,
                    hash: None,
                    private: None,
                    original: &jumbf[63..88],
                },
                child_boxes: vec!(),
                original: &jumbf[55..88],
            })),
            original: &jumbf,
        }
    );

    assert!(sbox.find_by_label("not_there").is_none());

    let dbox_as_child = sbox.child_boxes.first().unwrap();
    assert!(dbox_as_child.as_data_box().is_none());
    assert_eq!(
        dbox_as_child.as_super_box().unwrap(),
        &SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: None,
                requestable: false,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[63..88],
            },
            child_boxes: vec!(),
            original: &jumbf[55..88],
        }
    );
}

#[test]
fn data_box_sample() {
    let jumbf = hex!(
    "00000077" // box size
    "6a756d62" // box type = 'jumb'
        "00000028" // box size
        "6a756d64" // box type = 'jumd'
        "6332637300110010800000aa00389b71" // UUID
        "03" // toggles
        "633270612e7369676e617475726500" // label
        // ----
        "00000047" // box size
        "75756964" // box type = 'uuid'
        "6332637300110010800000aa00389b717468697320776f756c64206e6f726d616c6c792062652062696e617279207369676e617475726520646174612e2e2e" // data (type unknown)
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa.signature"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..48],
            },
            child_boxes: vec!(ChildBox::DataBox(DataBox {
                tbox: BoxType(*b"uuid"),
                data: &[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ],
                original: &jumbf[48..119],
            })),
            original: &jumbf,
        }
    );

    let uuid_box = sbox.data_box().unwrap();
    assert_eq!(uuid_box.offset_within_superbox(&sbox).unwrap(), 56);

    let dbox_as_child = sbox.child_boxes.first().unwrap();
    assert!(dbox_as_child.as_super_box().is_none());
    assert_eq!(
        dbox_as_child.as_data_box().unwrap(),
        &DataBox {
            tbox: BoxType(*b"uuid"),
            data: &[
                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105, 115,
                32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121, 32, 98,
                101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114,
                101, 32, 100, 97, 116, 97, 46, 46, 46,
            ],
            original: &jumbf[48..119],
        }
    );
}

#[test]
fn complex_example() {
    let jumbf = hex!(
        "00000267" // box size
        "6a756d62" // box type = 'jumb'
            "0000001e" // box size
            "6a756d64" // box type = 'jumd'
            "6332706100110010800000aa00389b71" // UUID
            "03" // toggles
            "6332706100" // label = "c2pa"
            // ---
            "00000241" // box size
            "6a756d62" // box type = 'jumb'
                "00000024" // box size
                "6a756d64" // box type = 'jumd'
                "63326d6100110010800000aa00389b71" // UUID
                "03" // toggles
                "63622e61646f62655f3100" // label = "cb.adobe_1"
                // ---
                "0000008f" // box size
                "6a756d62" // box type = 'jumb'
                    "00000029" // box size
                    "6a756d64" // box type = 'jumd'
                    "6332617300110010800000aa00389b71" // UUID
                    "03" // toggles
                    "633270612e617373657274696f6e7300" // label = "c2pa.assertions"
                    // ---
                    "0000005e" // box size
                    "6a756d62" // box type = 'jumb'
                        "0000002d" // box size
                        "6a756d64" // box type = 'jumd'
                        "6a736f6e00110010800000aa00389b71" // UUID
                        "03" // toggles
                        "633270612e6c6f636174696f6e2e62726f616400"
                            // label = "c2pa.location.broad"
                        // ---
                        "00000029" // box size
                        "6a736f6e" // box type = 'json'
                        "7b20226c6f636174696f6e223a20224d61726761"
                        "746520436974792c204e4a227d" // payload (JSON)
                // ---
                "0000010f" // box size
                "6a756d62" // box type = 'jumb'
                    "00000024" // box size
                    "6a756d64" // box type = 'jumd'
                    "6332636c00110010800000aa00389b71" // UUID
                    "03" // toggles
                    "633270612e636c61696d00" // label = "c2pa.claim"
                    // ---
                    "000000e3" // box size
                    "6a736f6e" // box type = 'json'
                    "7b0a2020202020202020202020202272"
                    "65636f7264657222203a202250686f74"
                    "6f73686f70222c0a2020202020202020"
                    "20202020227369676e61747572652220"
                    "3a202273656c66236a756d62663d735f"
                    "61646f62655f31222c0a202020202020"
                    "20202020202022617373657274696f6e"
                    "7322203a205b0a202020202020202020"
                    "202020202020202273656c66236a756d"
                    "62663d61735f61646f62655f312f6332"
                    "70612e6c6f636174696f6e2e62726f61"
                    "643f686c3d3736313432424436323336"
                    "3346220a202020202020202020202020"
                    "5d0a20202020202020207d" // payload (JSON)
                // ---
                "00000077" // box size
                "6a756d62" // box type = 'jumb'
                    "00000028" // box size
                    "6a756d64" // box type = 'jumd'
                    "6332637300110010800000aa00389b71" // UUID
                    "03" // toggles
                    "633270612e7369676e617475726500" // label = "c2pa.signature"
                    // ---
                    "00000047" // box size
                    "75756964" // box type = 'uuid'
                    "6332637300110010800000aa00389b71"
                    "7468697320776f756c64206e6f726d61"
                    "6c6c792062652062696e617279207369"
                    "676e617475726520646174612e2e2e"
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..38],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: &[99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some("cb.adobe_1"),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &jumbf[46..82],
                },
                child_boxes: vec!(
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &[
                                99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                            ],
                            label: Some("c2pa.assertions",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[90..131],
                        },
                        child_boxes: vec![ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: &[
                                    106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                    113,
                                ],
                                label: Some("c2pa.location.broad",),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: &jumbf[139..184],
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"json"),
                                data: &[
                                    123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58, 32,
                                    34, 77, 97, 114, 103, 97, 116, 101, 32, 67, 105, 116, 121, 44,
                                    32, 78, 74, 34, 125,
                                ],
                                original: &jumbf[184..225],
                            },),],
                            original: &jumbf[131..225],
                        },),],
                        original: &jumbf[82..225],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &[
                                99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                            ],
                            label: Some("c2pa.claim",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[233..269],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"json"),
                            data: &[
                                123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 114,
                                101, 99, 111, 114, 100, 101, 114, 34, 32, 58, 32, 34, 80, 104, 111,
                                116, 111, 115, 104, 111, 112, 34, 44, 10, 32, 32, 32, 32, 32, 32,
                                32, 32, 32, 32, 32, 32, 34, 115, 105, 103, 110, 97, 116, 117, 114,
                                101, 34, 32, 58, 32, 34, 115, 101, 108, 102, 35, 106, 117, 109, 98,
                                102, 61, 115, 95, 97, 100, 111, 98, 101, 95, 49, 34, 44, 10, 32,
                                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 97, 115, 115, 101,
                                114, 116, 105, 111, 110, 115, 34, 32, 58, 32, 91, 10, 32, 32, 32,
                                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 115, 101,
                                108, 102, 35, 106, 117, 109, 98, 102, 61, 97, 115, 95, 97, 100,
                                111, 98, 101, 95, 49, 47, 99, 50, 112, 97, 46, 108, 111, 99, 97,
                                116, 105, 111, 110, 46, 98, 114, 111, 97, 100, 63, 104, 108, 61,
                                55, 54, 49, 52, 50, 66, 68, 54, 50, 51, 54, 51, 70, 34, 10, 32, 32,
                                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 93, 10, 32, 32, 32, 32, 32,
                                32, 32, 32, 125,
                            ],
                            original: &jumbf[269..496],
                        },),],
                        original: &jumbf[225..496],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &[
                                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                            ],
                            label: Some("c2pa.signature",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[504..544],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"uuid"),
                            data: &[
                                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                                116, 104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114,
                                109, 97, 108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114,
                                121, 32, 115, 105, 103, 110, 97, 116, 117, 114, 101, 32, 100, 97,
                                116, 97, 46, 46, 46,
                            ],
                            original: &jumbf[544..615],
                        },),],
                        original: &jumbf[496..615],
                    },),
                ),
                original: &jumbf[38..615],
            })),
            original: &jumbf,
        }
    );

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature"),
        Some(&SuperBox {
            desc: DescriptionBox {
                uuid: &[99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa.signature",),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[504..544],
            },
            child_boxes: vec![ChildBox::DataBox(DataBox {
                tbox: BoxType(*b"uuid"),
                data: &[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ],
                original: &jumbf[544..615],
            },),],
            original: &jumbf[496..615],
        })
    );

    assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature")
            .and_then(|sig| sig.data_box()),
        Some(&DataBox {
            tbox: BoxType(*b"uuid"),
            data: &[
                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105, 115,
                32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121, 32, 98,
                101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114,
                101, 32, 100, 97, 116, 97, 46, 46, 46,
            ],
            original: &jumbf[544..615],
        })
    );

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature")
            .and_then(|sig| sig.data_box())
            .and_then(|sig| sig.offset_within_superbox(&sbox))
            .unwrap(),
        552
    );

    assert_eq!(sbox.data_box(), None);
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
        SuperBox::from_slice(&jumbf).unwrap_err(),
        Error::InvalidSuperBoxType(BoxType(*b"jumc"))
    );
}

#[test]
fn find_by_label_avoids_confict() {
    let jumbf = hex!(
        "00000093" // box size
        "6a756d62" // box type = 'jumb'
            "0000002f" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e7375706572626f785f64617461626f7800" // label
            // ------
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e64617461626f7800" // label = "test.databox"
            // ------
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e64617461626f7800" // label = "test.databox"
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.superbox_databox"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..55],
            },
            child_boxes: vec!(
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: &[0; 16],
                        label: Some("test.databox"),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &jumbf[63..101],
                    },
                    child_boxes: vec!(),
                    original: &jumbf[55..101],
                }),
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: &[0; 16],
                        label: Some("test.databox"),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &jumbf[109..147],
                    },
                    child_boxes: vec!(),
                    original: &jumbf[101..147],
                })
            ),
            original: &jumbf,
        }
    );

    assert_eq!(sbox.find_by_label("test.databox"), None);
}

#[test]
fn find_by_label_skips_non_requestable_boxes() {
    let jumbf = hex!(
        "00000093" // box size
        "6a756d62" // box type = 'jumb'
            "0000002f" // box size
            "6a756d64" // box type = 'jumd'
            "00000000000000000000000000000000" // UUID
            "03" // toggles
            "746573742e7375706572626f785f64617461626f7800" // label
            // ------
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "02" // toggles
                "746573742e64617461626f7800" // label = "test.databox"
            // ------
            "0000002e" // box size
            "6a756d62" // box type = 'jumb'
                "00000026" // box size
                "6a756d64" // box type = 'jumbd'
                "00000000000000000000000000000000" // UUID
                "03" // toggles
                "746573742e64617461626f7a00" // label = "test.databoz"
    );

    let (rem, sbox) = SuperBox::from_slice(&jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.superbox_databox"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..55],
            },
            child_boxes: vec!(
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: &[0; 16],
                        label: Some("test.databox"),
                        requestable: false,
                        id: None,
                        hash: None,
                        private: None,
                        original: &jumbf[63..101],
                    },
                    child_boxes: vec!(),
                    original: &jumbf[55..101],
                }),
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: &[0; 16],
                        label: Some("test.databoz"),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &jumbf[109..147],
                    },
                    child_boxes: vec!(),
                    original: &jumbf[101..147],
                })
            ),
            original: &jumbf,
        }
    );

    assert_eq!(sbox.find_by_label("test.databox"), None);

    assert_eq!(
        sbox.find_by_label("test.databoz"),
        Some(&SuperBox {
            desc: DescriptionBox {
                uuid: &[0; 16],
                label: Some("test.databoz"),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[109..147],
            },
            child_boxes: vec!(),
            original: &jumbf[101..147],
        })
    );
}

#[test]
fn parse_c2pa_manifest() {
    let jumbf = include_bytes!("../fixtures/C.c2pa");

    let (rem, sbox) = SuperBox::from_slice(jumbf).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: &hex!("63 32 70 61 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                label: Some("c2pa",),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &jumbf[8..38],
            },
            child_boxes: vec![ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: &hex!("63 32 6d 61 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                    label: Some("contentauth:urn:uuid:021b555e-5e02-4074-b444-43d7919d89b9",),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &jumbf[46..129],
                },
                child_boxes: vec![
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &hex!("63 32 61 73 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                            label: Some("c2pa.assertions",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[137..178],
                        },
                        child_boxes: vec![
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: &hex!("40 cb 0c 32 bb 8a 48 9d a7 0b 2a d6 f4 7f 43 69"),
                                    label: Some("c2pa.thumbnail.claim.jpeg",),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: &jumbf[186..237],
                                },
                                child_boxes: vec![
                                    ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"bfdb"),
                                        data: &jumbf[245..257],
                                        original: &jumbf[237..257],
                                    },),
                                    ChildBox::DataBox(DataBox {
                                        tbox: BoxType(*b"bidb"),
                                        data: &jumbf[265..31976],
                                        original: &jumbf[257..31976],
                                    },),
                                ],
                                original: &jumbf[178..31976],
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: &hex!("6a 73 6f 6e 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                    label: Some("stds.schema-org.CreativeWork",),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: Some(DataBox {
                                        tbox: BoxType(*b"c2sh"),
                                        data: &jumbf[32046..32062],
                                        original: &jumbf[32038..32062],
                                    },),
                                    original: &jumbf[31984..32062],
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"json"),
                                    data: &jumbf[32070..32179],
                                    original: &jumbf[32062..32179],
                                },),],
                                original: &jumbf[31976..32179],
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: &hex!("63 62 6f 72 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                    label: Some("c2pa.actions",),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: &jumbf[32187..32225],
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"cbor"),
                                    data: &jumbf[32233..32311],
                                    original: &jumbf[32225..32311],
                                },),],
                                original: &jumbf[32179..32311],
                            },),
                            ChildBox::SuperBox(SuperBox {
                                desc: DescriptionBox {
                                    uuid: &hex!("63 62 6f 72 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                                    label: Some("c2pa.hash.data",),
                                    requestable: true,
                                    id: None,
                                    hash: None,
                                    private: None,
                                    original: &jumbf[32319..32359],
                                },
                                child_boxes: vec![ChildBox::DataBox(DataBox {
                                    tbox: BoxType(*b"cbor"),
                                    data: &jumbf[32367..32482],
                                    original: &jumbf[32359..32482],
                                },),],
                                original: &jumbf[32311..32482],
                            },),
                        ],
                        original: &jumbf[129..32482],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &hex!("63 32 63 6c 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                            label: Some("c2pa.claim",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[32490..32526],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"cbor"),
                            data: &jumbf[32534..33166],
                            original: &jumbf[32526..33166],
                        },),],
                        original: &jumbf[32482..33166],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: &hex!("63 32 63 73 00 11 00 10 80 00 00 aa 00 38 9b 71"),
                            label: Some("c2pa.signature",),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &jumbf[33174..33214],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"cbor"),
                            data: &jumbf[33222..46948],
                            original: &jumbf[33214..46948],
                        },),],
                        original: &jumbf[33166..46948],
                    },),
                ],
                original: &jumbf[38..46948],
            },),],
            original: &jumbf[0..46948],
        }
    );
}
