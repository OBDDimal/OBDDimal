use criterion::{criterion_group, criterion_main, Criterion};

use obbdimal::{bdd::bdd_ds::{Bdd, InputFormat}, input::static_ordering::StaticOrdering};
use obbdimal::input::parser::ParserSettings;

fn build_sandwich_bdd() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/sandwich.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let _mgr = Bdd::from_format(&data, InputFormat::CNF, ParserSettings::default(), StaticOrdering::NONE).unwrap();
}

fn build_berkeley_bdd() {
    // Read data from a dimacs file.
    let data = std::fs::read_to_string("examples/assets/berkeleydb.dimacs").unwrap();
    // Create a BDD from input data (interpreted as dimacs cnf).
    let _mgr = Bdd::from_format(&data, InputFormat::CNF, ParserSettings::default(), StaticOrdering::NONE).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Build-BDDs-From-CNF");
    group.sample_size(10);
    group.bench_function("Build BDD from sandwich.dimacs", |b| {
        b.iter(|| build_sandwich_bdd())
    });
    group.bench_function("Build BDD from berkeley.dimacs", |b| {
        b.iter(|| build_berkeley_bdd())
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
