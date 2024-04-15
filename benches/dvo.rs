use std::fs;

use criterion::{criterion_group, Criterion};
use obddimal::{core::bdd_manager::DDManager, misc::static_ordering};

pub fn berkeleydb_sift_all(c: &mut Criterion) {
    let mut cnf = dimacs::parse_dimacs(
        &fs::read_to_string("examples/berkeleydb.dimacs").expect("Failed to read dimacs file."),
    )
    .expect("Failed to parse dimacs file.");
    let order = Some(static_ordering::keep(&cnf));
    let (man, bdd) = DDManager::from_instance(&mut cnf, order, Default::default()).unwrap();

    let mut group = c.benchmark_group("sifting");
    group.sample_size(10);

    group.bench_function("berkeleydb_sift_all", |b| {
        let mut man = man.clone();
        let mut bdd = bdd;
        b.iter(|| {
            bdd = man.sift_all_vars(bdd, false, None);
        })
    });

    group.finish();
}

criterion_group!(dvo, berkeleydb_sift_all);
