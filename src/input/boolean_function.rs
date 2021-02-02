/// A `Symbol` represents either a `BooleanFunction`,
/// a terminal symbol containing the index of a variable `Posterminal(u32)`
/// or a terminal symbol containing the index of a negated variable `Negterminal(u32)`
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Symbol {
    Posterminal(u32),
    Negterminal(u32),
    Function(BooleanFunction),
}

impl From<i32> for Symbol {
    fn from(i: i32) -> Self {
        if i >= 0 {
            Self::Posterminal(i as u32)
        } else {
            Self::Negterminal(-i as u32)
        }
    }
}

/// Represents all the operations currently supported by the `BooleanFunction` struct.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Operator {
    And,
    Or,
}

/// Represents a Boolean function.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BooleanFunction {
    pub op: Operator,
    pub lhs: Box<Symbol>,
    pub rhs: Box<Symbol>,
}

impl BooleanFunction {
    /// Creates a `BooleanFunction` struct containing an Operaton (currently Boolean `And` and Boolean `Or`) `op`,
    /// the left hand side of the function `lhs`,
    /// and the right hand side of the function `rhs`.
    fn new(op: Operator, lhs: Symbol, rhs: Symbol) -> BooleanFunction {
        BooleanFunction {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    /// Creates a `Symbol` out of a `Vec<Vec<i32>>` where every 'inside' `Vec<i32>` represents a
    /// single clause and every literal in this clause is a disjunction (OR) of literals.
    /// The returning `Symbol` is a conjunction of those clauses.
    pub fn new_from_cnf_formula(inp: Vec<Vec<i32>>) -> Symbol {
        let inp = inp
            .iter()
            .map(|x| x.iter().map(|&x| Symbol::from(x)).collect::<Vec<Symbol>>())
            .collect::<Vec<Vec<Symbol>>>();
        Self::new_cnf_formula_rec(inp)
    }

    fn new_cnf_formula_rec(inp: Vec<Vec<Symbol>>) -> Symbol {
        if inp.len() == 1 {
            return Self::new_cnf_term_rec(inp[0].clone());
        }

        let mut inp = inp;
        let fst = inp[0].clone();
        inp.reverse();
        inp.pop();
        inp.reverse();

        Symbol::Function(Self::new(
            Operator::And,
            Self::new_cnf_term_rec(fst),
            Self::new_cnf_formula_rec(inp),
        ))
    }

    fn new_cnf_term_rec(inp: Vec<Symbol>) -> Symbol {
        if inp.len() == 1 {
            return inp[0].clone();
        }

        let mut inp = inp;

        let fst = inp[0].clone();
        inp.reverse();
        inp.pop();
        inp.reverse();

        Symbol::Function(Self::new(Operator::Or, fst, Self::new_cnf_term_rec(inp)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn generate_easy_formula() {
        let input = vec![vec![1]];
        let output = super::BooleanFunction::new_from_cnf_formula(input);

        assert_eq!(output, super::Symbol::Posterminal(1))
    }

    #[test]
    fn generate_complex_formula() {
        use super::BooleanFunction;
        use super::Operator;
        use super::Symbol;

        let input = vec![vec![1, 2, 3], vec![2, 3, 4]];
        let output = super::BooleanFunction::new_from_cnf_formula(input);

        assert_eq!(
            output,
            Symbol::Function(BooleanFunction {
                op: Operator::And,
                lhs: Box::new(Symbol::Function(BooleanFunction {
                    op: Operator::Or,
                    lhs: Box::new(Symbol::Posterminal(1)),
                    rhs: Box::new(Symbol::Function(BooleanFunction {
                        op: Operator::Or,
                        lhs: Box::new(Symbol::Posterminal(2)),
                        rhs: Box::new(Symbol::Posterminal(3))
                    }))
                })),
                rhs: Box::new(Symbol::Function(BooleanFunction {
                    op: Operator::Or,
                    lhs: Box::new(Symbol::Posterminal(2)),
                    rhs: Box::new(Symbol::Function(BooleanFunction {
                        op: Operator::Or,
                        lhs: Box::new(Symbol::Posterminal(3)),
                        rhs: Box::new(Symbol::Posterminal(4))
                    }))
                }))
            })
        )
    }
}
