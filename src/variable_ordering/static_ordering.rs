use crate::input::parser::Cnf;

// TODO: Make this not a Cnf, create a Trait instead.
pub fn force_heuristic(cnf: Cnf) -> Cnf {
    let current_ordering = cnf.terms.clone();

    for _ in 0..10 {
        // calculate all centers of gravity
        let centers_of_gravity = current_ordering
            .iter()
            .map(|y| y.iter().sum::<i32>() / y.len() as i32)
            .collect::<Vec<i32>>();
    }

    cnf
}
