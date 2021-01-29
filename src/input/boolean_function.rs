#[derive(Debug, Clone)]
pub enum Symbol {
    Posterminal(i32),
    Negterminal(i32),
    Function(BooleanFunction),
}

impl From<i32> for Symbol {
    fn from(i: i32) -> Self {
        if i >= 0 {
            Self::Posterminal(i)
        } else {
            Self::Negterminal(i)
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Operator {
    And, Or
}

#[derive(Debug, Clone)]
pub struct BooleanFunction {
    pub op: Operator,
    pub lhs: Box<Symbol>,
    pub rhs: Box<Symbol>,
}

impl BooleanFunction {
    /// Creates a Boolean Function out of a `Vec<Vec<i32>>`.
    pub fn new_cnf_formula(inp: Vec<Vec<i32>>) -> Symbol {
        let inp = inp.iter().map(|x| x.iter().map(|x| Symbol::from(*x)).collect::<Vec<Symbol>>()).collect::<Vec<Vec<Symbol>>>();
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
        if inp.len() == 1 { return Self::new_cnf_term_rec(inp[0].clone()); }
        
        let mut inp = inp;
        let fst = inp[0].clone();
        inp.reverse();
        inp.pop();
        inp.reverse();

        Symbol::Function(Self::new(Operator::And, Self::new_cnf_term_rec(fst), Self::new_cnf_formula_rec(inp)))
    }

    fn new_cnf_term(inp: Vec<i32>) -> Symbol {
        let inp = inp.into_iter().map(Symbol::from).collect::<Vec<_>>();
        Self::new_cnf_term_rec(inp)
    }

    fn new_cnf_term_rec(inp: Vec<Symbol>) -> Symbol {
        if inp.len() == 1 { return inp[0].clone(); }

        let mut inp = inp;

        let fst = inp[0].clone();
        inp.reverse();
        inp.pop();
        inp.reverse();

        Symbol::Function(Self::new(Operator::Or, fst, Self::new_cnf_term_rec(inp)))
    }
}
