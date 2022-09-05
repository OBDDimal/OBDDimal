use criterion::{criterion_main};

mod bdd_creation;
mod sat;
mod dvo;

criterion_main!(bdd_creation::bdd_creation, sat::sat, dvo::dvo);
