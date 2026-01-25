use std::{cell::RefCell, io::Cursor, rc::Rc};

use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use jumbf::parser::DataBox;

const C2PA_MANIFEST_STORE: &[u8; 46948] = include_bytes!("../src/tests/fixtures/C.c2pa");

pub fn parse_c2pa_cursor(c: &mut Criterion) {
    c.bench_function("parse C2PA from cursor", |b| {
        b.iter(|| {
            let cursor = Cursor::new(C2PA_MANIFEST_STORE.to_vec());
            let reader = Rc::new(RefCell::new(cursor));
            DataBox::from_reader(black_box(reader)).unwrap()
        });
    });
}

criterion_group!(benches, parse_c2pa_cursor);
criterion_main!(benches);
