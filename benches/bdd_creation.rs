use criterion::{criterion_group, criterion_main, Criterion};

use obddimal::{bdd_manager::DDManager, dimacs};

pub fn sandwich_create_benchmark(c: &mut Criterion) {
    let cnf = dimacs::parse_dimacs("examples/sandwich.dimacs");
    c.bench_function("sandwich.dimacs bdd creation", |b| {
        b.iter(|| DDManager::from_instance(&mut cnf.clone(), None))
    });
}

criterion_group!(benches, sandwich_create_benchmark);
criterion_main!(benches);
