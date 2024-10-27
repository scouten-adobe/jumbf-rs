use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use jumbf::parser::DataBox;

const C2PA_MANIFEST_STORE: &[u8] = include_bytes!("../src/tests/fixtures/C.c2pa").as_slice();

pub fn parse_c2pa(c: &mut Criterion) {
    c.bench_function("parse sample C2PA Manifest Store", |b| {
        b.iter(|| DataBox::from_source(black_box(C2PA_MANIFEST_STORE)).unwrap());
    });
}

criterion_group!(benches, parse_c2pa);
criterion_main!(benches);
