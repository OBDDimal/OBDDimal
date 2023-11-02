use criterion::{criterion_group, Criterion};
use obddimal::{core::bdd_manager::DDManager, misc::static_ordering};
use std::fs;

pub fn berkeleydb_count_active(c: &mut Criterion) {
    let mut cnf = dimacs::parse_dimacs(
        &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
    )
    .expect("Failed to parse dimacs file.");
    let order = Some(static_ordering::keep(&cnf));
    let (man, bdd) = DDManager::from_instance(&mut cnf, order, Default::default()).unwrap();

    c.bench_function("berkeleydb_count_active", |b| {
        b.iter(|| man.count_active(bdd))
    });
}

criterion_group!(sat, berkeleydb_count_active);
