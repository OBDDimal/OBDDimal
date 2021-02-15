use crate::{parser::Cnf, variable_ordering::static_ordering::force};

/// Currently supported heurisitcs for static variable ordering.
pub enum StaticOrdering {
    NONE,
    FORCE,
}

/// Applies a given static variable ordering heuristic to a given `Cnf`
/// and returns a new `Cnf` with the newly calculated order.
/// Currently the following heuristics are supported:
/// -NONE
/// -FORCE
pub fn apply_heuristic(cnf: Cnf, heuristic: StaticOrdering) -> Cnf {
    match heuristic {
        StaticOrdering::NONE => cnf,
        StaticOrdering::FORCE => apply_force(cnf),
    }
}

fn apply_force(cnf: Cnf) -> Cnf {
    let terms = cnf.terms.clone();
    let term_count = cnf.term_count;
    let variable_count = cnf.varibale_count;
    let (ord, _) = force(cnf);

    Cnf {
        varibale_count: variable_count,
        term_count: term_count,
        terms: terms,
        order: ord,
    }
}
