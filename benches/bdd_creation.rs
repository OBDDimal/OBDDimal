use concat_idents::concat_idents;
use criterion::{criterion_group, Criterion};
use obddimal::{build::from_dimacs::dimacs, core::bdd_manager::DDManager, misc::static_ordering};

macro_rules! bdd_create_benchmark {
    ($name:ident) => {
        concat_idents!(fn_name = $name, _create_benchmark {
            pub fn fn_name(c: &mut Criterion) {
                let cnf = dimacs::parse_dimacs(concat!("examples/", stringify!($name), ".dimacs"));
                let order = Some(static_ordering::keep(&cnf));
                c.bench_function(concat!(stringify!($name), ".dimacs bdd creation"), |b| {
                    b.iter(|| DDManager::from_instance(&mut cnf.clone(), order.clone(), Default::default()))
                });
            }
        });
    };
}

bdd_create_benchmark!(sandwich);
bdd_create_benchmark!(berkeleydb);
// bdd_create_benchmark!(busybox);

criterion_group!(
    bdd_creation,
    sandwich_create_benchmark,
    berkeleydb_create_benchmark,
    // busybox_create_benchmark,
);
