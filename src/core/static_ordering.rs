use super::dimacs::Instance;

use rand::seq::SliceRandom;
use rand::thread_rng;

#[allow(dead_code)]
pub fn keep(instance: &Instance) -> Vec<u32> {
    let mut order: Vec<u32> = (1..instance.no_variables + 1).collect();

    order.insert(0, (order.len() + 1) as u32);
    order
}

#[allow(dead_code)]
pub fn rand(instance: &Instance) -> Vec<u32> {
    let mut order: Vec<u32> = (1..instance.no_variables + 1).collect();

    order.shuffle(&mut thread_rng());

    order.insert(0, (order.len() + 1) as u32);
    order
}

#[allow(dead_code)]
pub fn force(instance: &Instance) -> Vec<u32> {
    let mut order: Vec<u32> = (1..instance.no_variables + 1).collect();

    order.shuffle(&mut thread_rng());
    order.insert(0, (order.len() + 1) as u32);

    let mut span: Option<u32> = None;

    let mut converged = false;

    let mut n = 0;

    while !converged && n < 1000 {
        n += 1;

        let mut tpos: Vec<f64> = vec![0.0; (instance.no_variables + 1) as usize];
        let mut degree: Vec<u32> = vec![0; (instance.no_variables + 1) as usize];

        for clause in &instance.clauses {
            let cog: f64 = calc_center_of_gravity(clause, &order);

            for x in clause {
                let y = x.abs() as usize;
                tpos[y] += cog;
                degree[y] += 1;
            }
        }

        for x in 1..instance.no_variables + 1 {
            let y = x as usize;
            tpos[y] /= degree[y] as f64;
        }

        // println!("{:?}", tpos);

        order = (1..instance.no_variables + 1).collect::<Vec<u32>>();

        // println!("{:?}", order);

        order.sort_by(|x, y| tpos[*x as usize].partial_cmp(&tpos[*y as usize]).unwrap());
        // println!("{:?}", order);
        order.insert(0, (order.len() + 1) as u32);

        let span_new = calc_span(&instance.clauses, &order);

        if span.is_none() || span_new != span.unwrap() {
            span = Some(span_new);
        } else {
            converged = true;
        }

        println!("{:?}", span);
    }

    order
}

fn calc_center_of_gravity(clause: &Vec<i32>, order: &[u32]) -> f64 {
    let mut out = 0;
    for x in clause {
        out += order[x.abs() as usize];
    }

    out as f64 / clause.len() as f64
}

fn calc_span(clauses: &Vec<Vec<i32>>, order: &[u32]) -> u32 {
    let mut span = 0;

    for clause in clauses {
        let pos = clause
            .iter()
            .map(|x| order[x.abs() as usize] as u32)
            .collect::<Vec<u32>>();
        let min = pos.iter().min().unwrap();
        let max = pos.iter().max().unwrap();
        span += max - min;
    }

    span
}

#[allow(dead_code)]
fn order_dist(instance: &Instance) -> Vec<u32> {
    let n = (instance.no_variables + 1) as usize;
    let mut dists: Vec<Vec<u32>> = vec![vec![0; n]; n];
    for clause in &instance.clauses {
        for x_ in clause {
            let x = x_.abs() as usize;

            for y_ in clause {
                let y = y_.abs() as usize;
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
