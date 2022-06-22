use criterion::{criterion_group, criterion_main, Criterion};

use num_bigint::BigUint;

use obddimal::{bdd_manager::DDManager, dimacs::parse_dimacs};

fn build_verify_ssat(filepath: &str, target: &[u8]) {
    let expected = BigUint::parse_bytes(target, 10).unwrap();

    let mut instance = parse_dimacs(filepath);
    let (man, bdd) = DDManager::from_instance(&mut instance, None);

    assert_eq!(man.sat_count(bdd), expected);
}

fn sandwich_ssat() {
    build_verify_ssat("examples/sandwich.dimacs", b"2808")
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("ssat sandwich", |b| b.iter(|| sandwich_ssat()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
