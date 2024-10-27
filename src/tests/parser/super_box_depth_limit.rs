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
    parser::{ChildBox, DataBox, DescriptionBox, SuperBox},
    BoxType,
};

type TDataBox<'a> = DataBox<&'a [u8]>;

const JUMBF: &[u8] = hex!(
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
)
.as_slice();

#[test]
fn depth_limit_0() {
    let (sbox, rem) = SuperBox::from_source_with_depth_limit(JUMBF, 0).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[8..38],
            },
            child_boxes: vec!(ChildBox::DataBox(DataBox {
                tbox: BoxType(*b"jumb"),
                original: &JUMBF[38..615],
                data: &JUMBF[46..615],
            })),
            original: JUMBF,
        }
    );

    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

    let data_box = sbox.data_box().unwrap();

    assert_eq!(
        data_box,
        &DataBox {
            tbox: BoxType(*b"jumb"),
            original: &JUMBF[38..615],
            data: &JUMBF[46..615],
        }
    );

    let nested_box = SuperBox::from_data_box(data_box).unwrap();

    assert_eq!(
        nested_box,
        SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("cb.adobe_1".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[46..82],
            },
            child_boxes: vec!(
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some("c2pa.assertions".to_owned()),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &JUMBF[90..131],
                    },
                    child_boxes: vec![ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [
                                106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,
                            ],
                            label: Some("c2pa.location.broad".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[139..184],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"json"),
                            data: &[
                                123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58, 32, 34,
                                77, 97, 114, 103, 97, 116, 101, 32, 67, 105, 116, 121, 44, 32, 78,
                                74, 34, 125,
                            ],
                            original: &JUMBF[184..225],
                        },),],
                        original: &JUMBF[131..225],
                    },),],
                    original: &JUMBF[82..225],
                },),
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some("c2pa.claim".to_owned()),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &JUMBF[233..269],
                    },
                    child_boxes: vec![ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"json"),
                        data: &[
                            123, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 34, 114, 101,
                            99, 111, 114, 100, 101, 114, 34, 32, 58, 32, 34, 80, 104, 111, 116,
                            111, 115, 104, 111, 112, 34, 44, 10, 32, 32, 32, 32, 32, 32, 32, 32,
                            32, 32, 32, 32, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 32,
                            58, 32, 34, 115, 101, 108, 102, 35, 106, 117, 109, 98, 102, 61, 115,
                            95, 97, 100, 111, 98, 101, 95, 49, 34, 44, 10, 32, 32, 32, 32, 32, 32,
                            32, 32, 32, 32, 32, 32, 34, 97, 115, 115, 101, 114, 116, 105, 111, 110,
                            115, 34, 32, 58, 32, 91, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                            32, 32, 32, 32, 32, 32, 34, 115, 101, 108, 102, 35, 106, 117, 109, 98,
                            102, 61, 97, 115, 95, 97, 100, 111, 98, 101, 95, 49, 47, 99, 50, 112,
                            97, 46, 108, 111, 99, 97, 116, 105, 111, 110, 46, 98, 114, 111, 97,
                            100, 63, 104, 108, 61, 55, 54, 49, 52, 50, 66, 68, 54, 50, 51, 54, 51,
                            70, 34, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 93, 10, 32,
                            32, 32, 32, 32, 32, 32, 32, 125,
                        ],
                        original: &JUMBF[269..496],
                    },),],
                    original: &JUMBF[225..496],
                },),
                ChildBox::SuperBox(SuperBox {
                    desc: DescriptionBox {
                        uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                        label: Some("c2pa.signature".to_owned()),
                        requestable: true,
                        id: None,
                        hash: None,
                        private: None,
                        original: &JUMBF[504..544],
                    },
                    child_boxes: vec![ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"uuid"),
                        data: &[
                            99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116,
                            104, 105, 115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97,
                            108, 108, 121, 32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115,
                            105, 103, 110, 97, 116, 117, 114, 101, 32, 100, 97, 116, 97, 46, 46,
                            46,
                        ],
                        original: &JUMBF[544..615],
                    },),],
                    original: &JUMBF[496..615],
                },),
            ),
            original: &JUMBF[38..615],
        }
    );
}

#[test]
fn depth_limit_1() {
    let (sbox, rem) = SuperBox::from_source_with_depth_limit(JUMBF, 1).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[8..38],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some("cb.adobe_1".to_owned()),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &JUMBF[46..82],
                },
                child_boxes: vec!(
                    ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"jumb"),
                        original: &JUMBF[82..225],
                        data: &JUMBF[90..225],
                    }),
                    ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"jumb"),
                        original: &JUMBF[225..496],
                        data: &JUMBF[233..496],
                    }),
                    ChildBox::DataBox(DataBox {
                        tbox: BoxType(*b"jumb"),
                        original: &JUMBF[496..615],
                        data: &JUMBF[504..615],
                    }),
                ),
                original: &JUMBF[38..615],
            })),
            original: JUMBF,
        }
    );

    assert!(sbox.find_by_label("cb.adobe_1").is_some());
    assert!(sbox.find_by_label("cb.adobe_1/c2pa.signature").is_none());
    assert!(sbox.data_box().is_none());
}

#[test]
fn depth_limit_2() {
    let (sbox, rem) = SuperBox::from_source_with_depth_limit(JUMBF, 2).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[8..38],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some("cb.adobe_1".to_owned()),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &JUMBF[46..82],
                },
                child_boxes: vec!(
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.assertions".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[90..131],
                        },
                        child_boxes: vec![ChildBox::DataBox(DataBox {
                            tbox: BoxType(*b"jumb"),
                            data: &JUMBF[139..225],
                            original: &JUMBF[131..225],
                        },),],
                        original: &JUMBF[82..225],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.claim".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[233..269],
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
                            original: &JUMBF[269..496],
                        },),],
                        original: &JUMBF[225..496],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.signature".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[504..544],
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
                            original: &JUMBF[544..615],
                        },),],
                        original: &JUMBF[496..615],
                    },),
                ),
                original: &JUMBF[38..615],
            })),
            original: JUMBF,
        }
    );

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature"),
        Some(&SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa.signature".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[504..544],
            },
            child_boxes: vec![ChildBox::DataBox(DataBox {
                tbox: BoxType(*b"uuid"),
                data: &[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ],
                original: &JUMBF[544..615],
            },),],
            original: &JUMBF[496..615],
        })
    );

    assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature")
            .and_then(|sig| sig.data_box()),
        Some(&TDataBox {
            tbox: BoxType(*b"uuid"),
            data: &[
                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105, 115,
                32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121, 32, 98,
                101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114,
                101, 32, 100, 97, 116, 97, 46, 46, 46,
            ],
            original: &JUMBF[544..615],
        })
    );

    assert_eq!(sbox.data_box(), None);
}

#[test]
fn depth_limit_3() {
    let (sbox, rem) = SuperBox::from_source_with_depth_limit(JUMBF, 3).unwrap();
    assert!(rem.is_empty());

    assert_eq!(
        sbox,
        SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 112, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[8..38],
            },
            child_boxes: vec!(ChildBox::SuperBox(SuperBox {
                desc: DescriptionBox {
                    uuid: [99, 50, 109, 97, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                    label: Some("cb.adobe_1".to_owned()),
                    requestable: true,
                    id: None,
                    hash: None,
                    private: None,
                    original: &JUMBF[46..82],
                },
                child_boxes: vec!(
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 97, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.assertions".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[90..131],
                        },
                        child_boxes: vec![ChildBox::SuperBox(SuperBox {
                            desc: DescriptionBox {
                                uuid: [
                                    106, 115, 111, 110, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155,
                                    113,
                                ],
                                label: Some("c2pa.location.broad".to_owned()),
                                requestable: true,
                                id: None,
                                hash: None,
                                private: None,
                                original: &JUMBF[139..184],
                            },
                            child_boxes: vec![ChildBox::DataBox(DataBox {
                                tbox: BoxType(*b"json"),
                                data: &[
                                    123, 32, 34, 108, 111, 99, 97, 116, 105, 111, 110, 34, 58, 32,
                                    34, 77, 97, 114, 103, 97, 116, 101, 32, 67, 105, 116, 121, 44,
                                    32, 78, 74, 34, 125,
                                ],
                                original: &JUMBF[184..225],
                            },),],
                            original: &JUMBF[131..225],
                        },),],
                        original: &JUMBF[82..225],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 99, 108, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.claim".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[233..269],
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
                            original: &JUMBF[269..496],
                        },),],
                        original: &JUMBF[225..496],
                    },),
                    ChildBox::SuperBox(SuperBox {
                        desc: DescriptionBox {
                            uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                            label: Some("c2pa.signature".to_owned()),
                            requestable: true,
                            id: None,
                            hash: None,
                            private: None,
                            original: &JUMBF[504..544],
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
                            original: &JUMBF[544..615],
                        },),],
                        original: &JUMBF[496..615],
                    },),
                ),
                original: &JUMBF[38..615],
            })),
            original: JUMBF,
        }
    );

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature"),
        Some(&SuperBox {
            desc: DescriptionBox {
                uuid: [99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113,],
                label: Some("c2pa.signature".to_owned()),
                requestable: true,
                id: None,
                hash: None,
                private: None,
                original: &JUMBF[504..544],
            },
            child_boxes: vec![ChildBox::DataBox(DataBox {
                tbox: BoxType(*b"uuid"),
                data: &[
                    99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105,
                    115, 32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121,
                    32, 98, 101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116,
                    117, 114, 101, 32, 100, 97, 116, 97, 46, 46, 46,
                ],
                original: &JUMBF[544..615],
            },),],
            original: &JUMBF[496..615],
        })
    );

    assert_eq!(sbox.find_by_label("cb.adobe_1x/c2pa.signature"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signaturex"), None);
    assert_eq!(sbox.find_by_label("cb.adobe_1/c2pa.signature/blah"), None);

    assert_eq!(
        sbox.find_by_label("cb.adobe_1/c2pa.signature")
            .and_then(|sig| sig.data_box()),
        Some(&TDataBox {
            tbox: BoxType(*b"uuid"),
            data: &[
                99, 50, 99, 115, 0, 17, 0, 16, 128, 0, 0, 170, 0, 56, 155, 113, 116, 104, 105, 115,
                32, 119, 111, 117, 108, 100, 32, 110, 111, 114, 109, 97, 108, 108, 121, 32, 98,
                101, 32, 98, 105, 110, 97, 114, 121, 32, 115, 105, 103, 110, 97, 116, 117, 114,
                101, 32, 100, 97, 116, 97, 46, 46, 46,
            ],
            original: &JUMBF[544..615],
        })
    );

    assert_eq!(sbox.data_box(), None);
}
