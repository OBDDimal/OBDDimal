use criterion::{criterion_group, criterion_main, Criterion};

use obddimal::{bdd_manager::DDManager, dimacs};

pub fn sandwich_ssat_benchmark(c: &mut Criterion) {
    let cnf = dimacs::parse_dimacs("examples/sandwich.dimacs");
    c.bench_function("ssat sandwich bench", |b| {
        b.iter(|| DDManager::from_instance(&mut cnf.clone(), None))
    });
}

criterion_group!(benches, sandwich_ssat_benchmark);
criterion_main!(benches);
