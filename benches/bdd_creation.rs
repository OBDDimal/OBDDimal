use concat_idents::concat_idents;
use criterion::{criterion_group, criterion_main, Criterion};
use obddimal::{bdd_manager::DDManager, dimacs};

macro_rules! bdd_create_benchmark {
    ($name:ident) => {
        concat_idents!(fn_name = $name, _create_benchmark {
            pub fn fn_name(c: &mut Criterion) {
                let cnf = dimacs::parse_dimacs(concat!("examples/", stringify!($name), ".dimacs"));
                c.bench_function(concat!(stringify!($name), ".dimacs bdd creation"), |b| {
                    b.iter(|| DDManager::from_instance(&mut cnf.clone(), None))
                });
            }
        });
    };
}

bdd_create_benchmark!(sandwich);
bdd_create_benchmark!(berkeleydb);
bdd_create_benchmark!(busybox);

criterion_group!(
    benches,
    sandwich_create_benchmark,
    berkeleydb_create_benchmark,
    // busybox_create_benchmark,
);
criterion_main!(benches);
