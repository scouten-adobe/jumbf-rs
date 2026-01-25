use std::{cell::RefCell, fs::File, io::BufReader, rc::Rc};

use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use jumbf::parser::DataBox;

pub fn parse_c2pa_file(c: &mut Criterion) {
    c.bench_function("parse C2PA from file", |b| {
        b.iter(|| {
            let file = File::open("src/tests/fixtures/C.c2pa").unwrap();
            let buf_reader = BufReader::new(file);
            let reader = Rc::new(RefCell::new(buf_reader));
            DataBox::from_reader(black_box(reader)).unwrap()
        });
    });
}

criterion_group!(benches, parse_c2pa_file);
criterion_main!(benches);
