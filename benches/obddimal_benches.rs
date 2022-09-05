use criterion::criterion_main;

mod bdd_creation;
mod dvo;
mod sat;

criterion_main!(bdd_creation::bdd_creation, sat::sat, dvo::dvo);
