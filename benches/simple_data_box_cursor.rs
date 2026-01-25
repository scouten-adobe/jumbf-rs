use std::{cell::RefCell, io::Cursor, rc::Rc};

use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use hex_literal::hex;
use jumbf::parser::DataBox;

const SIMPLE_BOX: [u8; 38] = hex!(
    "00000026" // box size
    "6a756d64" // box type = 'jumd'
    "00000000000000000000000000000000" // UUID
    "03" // toggles
    "746573742e64657363626f7800" // label
);

pub fn simple_parse_cursor(c: &mut Criterion) {
    c.bench_function("simple data box from cursor", |b| {
        b.iter(|| {
            let cursor = Cursor::new(SIMPLE_BOX.to_vec());
            let reader = Rc::new(RefCell::new(cursor));
            DataBox::from_reader(black_box(reader)).unwrap()
        });
    });
}

criterion_group!(benches, simple_parse_cursor);
criterion_main!(benches);
