use criterion::{criterion_group, Criterion};
use obddimal::{bdd_manager::DDManager, dimacs, static_ordering};

pub fn berkeleydb_count_active(c: &mut Criterion) {
    let mut cnf = dimacs::parse_dimacs(concat!("examples/berkeleydb.dimacs"));
    let order = Some(static_ordering::keep(&cnf));
    let (man, bdd) = DDManager::from_instance(&mut cnf, order).unwrap();

    c.bench_function("berkeleydb_count_active", |b| {
        b.iter(|| man.count_active(bdd))
    });
}

criterion_group!(sat, berkeleydb_count_active);
