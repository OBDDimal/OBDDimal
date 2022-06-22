# OBDDimal

An experimental BDD library written in Rust.

## Running
This crate contains the `obddimal` library as well as an executable program of
the same name.
When running the executable, set the `RUST_LOG` variable to the desired logging
verbosity:
```console
RUST_LOG=info cargo run --release
```

## Benchmarking
Benchmarking is done using the [criterion.rs library](https://github.com/bheisler/criterion.rs).
To run the benchmarks, run `cargo bench`.
See [the criterion.rs user guide](https://bheisler.github.io/criterion.rs/book/criterion_rs.html)
for advanced options.

## Profiling
The [flamegraph crate](https://github.com/flamegraph-rs/flamegraph) simplifies
creation of flamegraphs.
This requires the flamegraph tool as well as `perf` to be installed on the
system.
To create a flamegraph for a benchmark, run:
```console
cargo flamegraph --bench bdd_creation -- --bench --profile-time 30
```
The first `--bench <name>` argument selects the benchmark, the second `--bench`
is needed to switch criterion from test into benchmark mode.
The `--profile-time` argument runs the benchmark for a set amount of time and
disables plotting and analysis.
