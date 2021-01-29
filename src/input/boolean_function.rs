/// A `Symbol` represents either a `BooleanFunction`, 
/// a terminal symbol containing the index of a variable `Posterminal(u32)`
/// or a terminal symbol containing the index of a negated variable `Negterminal(u32)`
#[derive(Debug, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub enum Operator {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct BooleanFunction {
    pub op: Operator,
    pub lhs: Box<Symbol>,
    pub rhs: Box<Symbol>,
}

impl BooleanFunction {
    /// Creates a `Symbol` out of a `Vec<Vec<i32>>` where every 'inside' `Vec<i32>` represents a 
    /// single clause and every literal in this clause is a disjunction (OR) of literals.
    /// The returning `Symbol` is a conjunction of those clauses.
    pub fn new_cnf_formula(inp: Vec<Vec<i32>>) -> Symbol {
        let inp = inp
            .iter()
            .map(|x| x.iter().map(|x| Symbol::from(*x)).collect::<Vec<Symbol>>())
            .collect::<Vec<Vec<Symbol>>>();
        Self::new_cnf_formula_rec(inp)
    }

    fn new(op: Operator, lhs: Symbol, rhs: Symbol) -> BooleanFunction {
        BooleanFunction {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
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

    fn new_cnf_term(inp: Vec<i32>) -> Symbol {
        let inp = inp.into_iter().map(Symbol::from).collect::<Vec<_>>();
        Self::new_cnf_term_rec(inp)
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
