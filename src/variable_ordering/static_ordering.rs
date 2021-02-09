use crate::input::parser::Cnf;

use std::collections::HashMap;

use rand::seq::SliceRandom;
use rand::thread_rng;

// TODO: Make this not a Cnf, create a Trait instead.
#[allow(unreachable_code, mutable_borrow_reservation_conflict)] // Only for cleaner console output, otherwise cargo spams the output with warnings.
pub fn force(cnf: Cnf) -> (Vec<i32>, i32) {
    let clauses = cnf.terms;
    let mut order: Vec<i32> = (1_i32..(cnf.varibale_count + 1) as i32).collect();

    order.shuffle(&mut thread_rng());

    let mut span = compute_span(&clauses, &order);

    for _ in 0..1000 {
        let mut cogs_v = HashMap::new();
        let span_old = span;

        for (_i, clause) in clauses.iter().enumerate() {
            let cogs = compute_cog(&clause, &order);

            for x in clause {
                let x = x.abs();
                if cogs_v.contains_key(&x) {
                    let (a, b) = cogs_v.get(&x).unwrap();
                    cogs_v.insert(x, (a + cogs, b + 1));
                } else {
                    cogs_v.insert(x, (cogs, 1));
                }
            }
        }

        let mut tlocs = vec![];

        for (key, value) in cogs_v.iter() {
            let (center, n) = value;
            tlocs.push((key, center / n));
        }

        tlocs.sort_by(|(_, b1), (_, b2)| b1.cmp(b2)); // switcharoo'ed cmp

        order = tlocs.iter().map(|(&a, _)| a as i32).collect();

        span = compute_span(&clauses, &order);

        if span == span_old {
            break;
        }
    }

    (order, span)
}

fn compute_cog(clause: &Vec<i32>, order: &Vec<i32>) -> i32 {
    let cog: i32 = clause
        .iter()
        .filter_map(|&x| order.iter().position(|&y| y.abs() == x))
        .map(|x| x as i32)
        .sum();

    cog / clause.len() as i32
}

fn compute_span(clauses: &Vec<Vec<i32>>, order: &Vec<i32>) -> i32 {
    let mut span = vec![];

    for clause in clauses.iter() {
        let indices: Vec<i32> = clause
            .iter()
            .filter_map(|&x| order.iter().position(|&y| y == x as i32))
            .map(|x| x as i32)
            .collect();

        let max = if let Some(&x) = indices.iter().max() {
            x
        } else {
            0
        };

        let min = if let Some(&x) = indices.iter().min() {
            x
        } else {
            0
        };

        let lspan = max - min;

        span.push(lspan);
    }

    span.iter().sum()
}
