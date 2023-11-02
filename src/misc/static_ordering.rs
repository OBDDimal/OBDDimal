//! Implementations of different static variable ordering strategies
use rand::{seq::SliceRandom, thread_rng};

use dimacs::{Clause, Instance};

#[allow(dead_code)]
pub fn keep(instance: &Instance) -> Vec<u32> {
    let num_vars = match instance {
        Instance::Cnf { num_vars, .. } => *num_vars as u32,
        _ => panic!("Unsupported dimacs format!"),
    };
    let mut order: Vec<u32> = (1..num_vars + 1).collect();

    order.insert(0, (order.len() + 1) as u32);
    order
}

#[allow(dead_code)]
pub fn rand(instance: &Instance) -> Vec<u32> {
    let num_vars = match instance {
        Instance::Cnf { num_vars, .. } => *num_vars as u32,
        _ => panic!("Unsupported dimacs format!"),
    };
    let mut order: Vec<u32> = (1..num_vars + 1).collect();

    order.shuffle(&mut thread_rng());

    order.insert(0, (order.len() + 1) as u32);
    order
}

#[allow(dead_code)]
pub fn force(instance: &Instance) -> Vec<u32> {
    let (num_vars, clauses) = match instance {
        Instance::Cnf { num_vars, clauses } => (*num_vars as u32, clauses),
        _ => panic!("Unsupported dimacs format!"),
    };
    let mut order: Vec<u32> = (1..num_vars + 1).collect();

    order.shuffle(&mut thread_rng());
    order.insert(0, (order.len() + 1) as u32);

    let mut span: Option<u32> = None;

    let mut converged = false;

    let mut n = 0;

    while !converged && n < 1000 {
        n += 1;

        let mut tpos: Vec<f64> = vec![0.0; (num_vars + 1) as usize];
        let mut degree: Vec<u32> = vec![0; (num_vars + 1) as usize];

        for clause in clauses.iter() {
            let cog: f64 = calc_center_of_gravity(clause, &order);

            for x in clause.lits().iter() {
                let y = x.var().to_u64() as usize;
                tpos[y] += cog;
                degree[y] += 1;
            }
        }

        for x in 1..num_vars + 1 {
            let y = x as usize;
            tpos[y] /= degree[y] as f64;
        }

        // println!("{:?}", tpos);

        order = (1..num_vars + 1).collect::<Vec<u32>>();

        // println!("{:?}", order);

        order.sort_by(|x, y| {
            tpos[*x as usize]
                .partial_cmp(&tpos[*y as usize])
                .unwrap_or(std::cmp::Ordering::Less)
        });
        // println!("{:?}", order);
        order.insert(0, (order.len() + 1) as u32);

        let span_new = calc_span(clauses, &order);

        if span.is_none() || span_new != span.unwrap() {
            span = Some(span_new);
        } else {
            converged = true;
        }
    }

    order
}

fn calc_center_of_gravity(clause: &Clause, order: &[u32]) -> f64 {
    let mut out = 0;
    for x in clause.lits().iter() {
        out += order[x.var().to_u64() as usize];
    }

    out as f64 / clause.len() as f64
}

fn calc_span(clauses: &[Clause], order: &[u32]) -> u32 {
    let mut span = 0;

    for clause in clauses.iter() {
        let pos = clause
            .lits()
            .iter()
            .map(|x| order[x.var().to_u64() as usize])
            .collect::<Vec<u32>>();
        let min = pos.iter().min().unwrap();
        let max = pos.iter().max().unwrap();
        span += max - min;
    }

    span
}

#[allow(dead_code)]
fn order_dist(instance: &Instance) -> Vec<u32> {
    let (num_vars, clauses) = match instance {
        Instance::Cnf { num_vars, clauses } => (*num_vars as usize, clauses),
        _ => panic!("Unsupported dimacs format!"),
    };
    let n = num_vars + 1;
    let mut dists: Vec<Vec<u32>> = vec![vec![0; n]; n];
    for clause in clauses.iter() {
        for x_ in clause.lits().iter() {
            let x = x_.var().to_u64() as usize;

            for y_ in clause.lits().iter() {
                let y = y_.var().to_u64() as usize;
                if x >= y {
                    break;
                }

                dists[x][y] += 1;
                dists[y][x] += 1;
            }
        }
    }

    println!(
        "{:?}",
        dists
            .iter()
            .enumerate()
            .map(|(i, x)| (i, x.iter().sum::<u32>()))
            .max_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap())
            .unwrap()
    );

    vec![0]
}
