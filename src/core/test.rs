#[cfg(test)]
pub mod tests {
    use std::fs;

    use num_bigint::BigUint;

    use crate::{
        core::bdd_node::{NodeID, VarID, ONE, ZERO},
        misc::hash_select::HashSet,
    };

    /// A manually constructed BDD plus truth table, allowing verification of
    /// any other BDD agains it for testing if it represents the same function.
    pub struct TestCase {
        ones: HashSet<Vec<VarID>>,
        pub man: DDManager,
        pub f: NodeID,
        pub nr_variables: usize,
    }

    impl TestCase {
        /// Construct testcase matching the "trivial" dimacs example
        /// (a + b + ~c) (a + ~b + c) (~a + ~b + c)
        /// ~a~b~c + ~abc + a~b~c + a~bc + abc
        /// ~b~c + bc + ac
        pub fn test_trivial() -> TestCase {
            let ones = HashSet::from_iter([
                vec![],
                vec![VarID(2), VarID(3)],
                vec![VarID(1)],
                vec![VarID(1), VarID(3)],
                vec![VarID(1), VarID(2), VarID(3)],
            ]);
            let mut man = DDManager::default();

            let mut f = ZERO.id;

            let nr_variables = 3;

            for clause in ones.iter() {
                let mut c = ONE.id;
                for var in 1..nr_variables + 1 {
                    let v = if clause.contains(&VarID(var)) {
                        man.ith_var(VarID(var))
                    } else {
                        man.nith_var(VarID(var))
                    };
                    c = man.and(c, v);
                }
                f = man.or(f, c);
            }

            TestCase {
                ones,
                man,
                f,
                nr_variables,
            }
        }

        pub fn random_1() -> TestCase {
            TestCase::from_truthtable(vec![
                [1, 0, 1, 0, 0, 1, 1, 1],
                [1, 0, 1, 1, 1, 1, 1, 0],
                [0, 1, 1, 1, 0, 0, 1, 1],
                [1, 0, 0, 0, 1, 1, 1, 1],
                [1, 0, 1, 0, 0, 1, 0, 1],
                [1, 1, 1, 0, 1, 0, 0, 1],
                [1, 1, 1, 1, 1, 0, 1, 1],
                [0, 1, 0, 0, 0, 1, 1, 0],
                [0, 1, 0, 0, 1, 1, 1, 0],
                [1, 1, 1, 0, 0, 0, 0, 1],
                [1, 1, 0, 0, 1, 0, 0, 1],
                [1, 1, 1, 0, 0, 0, 1, 1],
                [0, 1, 1, 1, 1, 0, 0, 0],
                [1, 0, 1, 1, 0, 1, 1, 1],
                [1, 1, 0, 1, 1, 0, 1, 0],
                [1, 1, 0, 0, 1, 1, 1, 0],
                [0, 0, 1, 1, 0, 0, 0, 0],
                [0, 0, 1, 1, 1, 0, 0, 0],
                [0, 1, 0, 1, 0, 1, 0, 0],
                [0, 1, 0, 0, 1, 1, 0, 1],
                [1, 1, 0, 0, 0, 0, 1, 0],
                [0, 1, 1, 0, 0, 0, 1, 0],
                [1, 1, 1, 0, 0, 0, 0, 0],
                [0, 1, 0, 0, 0, 0, 1, 1],
                [0, 0, 1, 0, 0, 1, 0, 0],
                [1, 1, 0, 0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0, 0, 0, 1],
                [0, 0, 0, 0, 1, 0, 1, 0],
                [0, 0, 1, 1, 1, 0, 1, 1],
                [0, 1, 0, 0, 1, 0, 1, 0],
                [0, 0, 0, 0, 0, 1, 0, 1],
                [0, 1, 1, 1, 1, 1, 0, 1],
                [0, 0, 1, 0, 0, 1, 0, 1],
                [1, 1, 1, 1, 0, 1, 0, 0],
                [1, 0, 1, 1, 1, 0, 1, 0],
                [1, 1, 1, 0, 1, 1, 1, 0],
                [1, 1, 0, 0, 1, 0, 1, 0],
                [0, 0, 0, 1, 0, 1, 1, 0],
                [0, 1, 1, 1, 0, 1, 1, 0],
                [0, 1, 1, 0, 1, 1, 0, 0],
                [1, 1, 0, 0, 1, 0, 0, 0],
                [1, 1, 1, 0, 0, 1, 1, 0],
                [1, 0, 1, 1, 1, 1, 0, 0],
                [1, 1, 1, 1, 0, 0, 1, 0],
                [0, 0, 1, 0, 0, 1, 1, 1],
                [0, 1, 1, 1, 0, 1, 1, 1],
                [1, 1, 0, 0, 1, 1, 0, 0],
                [1, 0, 1, 0, 0, 0, 0, 0],
                [1, 1, 0, 1, 0, 1, 1, 1],
                [1, 1, 0, 0, 1, 0, 1, 1],
                [0, 0, 0, 1, 1, 1, 1, 0],
                [1, 1, 0, 1, 1, 1, 0, 1],
                [0, 0, 0, 0, 0, 0, 1, 1],
                [0, 0, 0, 1, 0, 1, 0, 0],
                [0, 0, 1, 0, 1, 0, 1, 1],
                [0, 0, 1, 1, 0, 1, 1, 0],
                [0, 1, 1, 1, 1, 1, 1, 1],
                [0, 1, 1, 0, 1, 1, 0, 1],
                [1, 0, 0, 0, 0, 1, 1, 1],
                [0, 1, 0, 1, 0, 0, 1, 0],
                [1, 0, 0, 1, 1, 0, 1, 0],
                [1, 1, 0, 0, 1, 1, 1, 1],
                [1, 1, 0, 1, 0, 1, 0, 0],
                [1, 0, 0, 0, 0, 0, 0, 0],
                [1, 1, 1, 0, 1, 0, 1, 1],
                [1, 0, 0, 1, 0, 0, 0, 0],
                [0, 1, 1, 0, 0, 0, 0, 1],
                [0, 0, 0, 0, 1, 0, 0, 1],
                [0, 0, 1, 1, 1, 1, 0, 1],
                [0, 0, 0, 0, 0, 1, 0, 0],
                [1, 0, 1, 0, 1, 0, 1, 0],
                [1, 0, 1, 0, 1, 1, 1, 0],
                [1, 0, 1, 0, 0, 0, 0, 1],
                [1, 1, 0, 1, 1, 1, 0, 0],
                [0, 0, 1, 0, 1, 1, 0, 0],
                [0, 1, 0, 0, 0, 0, 1, 0],
                [0, 1, 0, 1, 1, 0, 1, 1],
                [1, 0, 1, 1, 1, 0, 0, 0],
                [1, 1, 1, 0, 1, 0, 0, 0],
                [1, 0, 0, 1, 0, 0, 1, 1],
                [1, 0, 0, 1, 0, 1, 0, 0],
                [0, 1, 0, 1, 1, 0, 0, 0],
                [0, 1, 0, 0, 1, 0, 0, 0],
                [0, 0, 0, 1, 0, 0, 0, 0],
                [1, 0, 1, 0, 0, 1, 1, 0],
                [0, 1, 0, 0, 0, 1, 0, 1],
                [0, 0, 1, 0, 1, 1, 0, 1],
                [0, 0, 1, 1, 1, 0, 0, 1],
                [1, 1, 0, 0, 0, 1, 1, 1],
                [0, 0, 0, 0, 0, 1, 1, 0],
                [0, 0, 1, 1, 1, 1, 1, 1],
                [1, 1, 1, 1, 1, 0, 0, 0],
                [1, 1, 0, 1, 0, 1, 1, 0],
                [0, 0, 0, 0, 1, 0, 1, 1],
                [1, 0, 1, 1, 0, 1, 0, 1],
                [0, 0, 0, 1, 1, 1, 0, 0],
                [1, 0, 1, 1, 0, 0, 1, 1],
                [1, 1, 0, 1, 0, 0, 0, 0],
                [0, 0, 1, 1, 1, 1, 1, 0],
                [1, 0, 0, 0, 1, 1, 1, 0],
                [0, 0, 0, 1, 1, 1, 0, 1],
                [0, 1, 1, 0, 1, 0, 0, 0],
                [1, 1, 1, 1, 1, 0, 0, 1],
                [1, 0, 1, 0, 1, 0, 1, 1],
                [1, 0, 0, 0, 0, 1, 0, 1],
                [0, 1, 1, 1, 0, 0, 0, 0],
                [0, 0, 1, 0, 0, 0, 1, 0],
                [0, 1, 0, 1, 1, 1, 0, 0],
                [1, 1, 1, 0, 1, 1, 1, 1],
                [1, 1, 0, 1, 1, 1, 1, 1],
                [1, 1, 1, 0, 1, 1, 0, 0],
                [0, 1, 1, 1, 1, 0, 1, 0],
                [1, 0, 1, 0, 0, 1, 0, 0],
                [1, 1, 1, 1, 0, 1, 1, 1],
                [1, 1, 0, 0, 0, 0, 0, 1],
                [1, 1, 0, 1, 0, 0, 1, 1],
                [1, 1, 0, 1, 1, 0, 1, 1],
                [0, 1, 0, 1, 0, 0, 0, 1],
                [0, 1, 1, 0, 1, 0, 0, 1],
                [1, 0, 0, 1, 0, 1, 0, 1],
                [0, 1, 0, 1, 0, 1, 1, 1],
                [0, 0, 1, 1, 1, 1, 0, 0],
                [0, 1, 1, 1, 1, 0, 1, 1],
                [1, 1, 1, 1, 0, 1, 1, 0],
                [1, 0, 0, 0, 0, 0, 1, 0],
                [1, 0, 0, 0, 1, 0, 0, 0],
                [1, 0, 0, 0, 0, 0, 1, 1],
                [0, 1, 0, 1, 1, 1, 0, 1],
                [0, 0, 1, 0, 1, 0, 0, 0],
                [1, 1, 0, 0, 1, 1, 0, 1],
                [0, 0, 1, 0, 0, 0, 1, 1],
                [0, 0, 0, 1, 0, 0, 1, 0],
            ])
        }

        fn from_truthtable<const N: usize>(table: Vec<[u8; N]>) -> TestCase {
            let mut clauses: HashSet<Vec<VarID>> = HashSet::default();

            for line in table {
                let mut clause = Vec::new();
                for (var, value) in line.iter().enumerate() {
                    if *value != 0 {
                        clause.push(VarID(var + 1))
                    }
                }

                clauses.insert(clause);
            }

            let mut man = DDManager::default();
            let mut f = ZERO.id;

            for clause in clauses.iter() {
                let mut c = ONE.id;
                for var in 1..N + 1 {
                    let v = if clause.contains(&VarID(var)) {
                        man.ith_var(VarID(var))
                    } else {
                        man.nith_var(VarID(var))
                    };
                    c = man.and(c, v);
                }
                f = man.or(f, c);
            }

            TestCase {
                ones: clauses,
                man,
                f,
                nr_variables: N,
            }
        }

        /// Test if a function in some other BDD matches this testcase
        #[must_use]
        pub fn verify_against(&self, other_man: &DDManager, other_f: NodeID) -> bool {
            for trues in self.ones.iter() {
                if !other_man.evaluate(other_f, trues) {
                    eprintln!("f({:?}=1) should be 1, but is not!", trues);
                    return false;
                }
            }

            if other_man.sat_count(other_f) != self.ones.len().into() {
                eprintln!(
                    "Sat count is {}, but should be {}",
                    other_man.sat_count(other_f),
                    self.ones.len()
                );
                fs::write("unittest_fail.dot", other_man.graphviz(other_f)).unwrap();
                return false;
            }

            true
        }
    }

    use crate::core::bdd_manager::DDManager;

    // Test that a testcase matches itself
    #[test]
    fn test_trivial_noop_trivial() {
        let testcase = TestCase::test_trivial();
        let man = testcase.man.clone();
        assert!(testcase.verify_against(&man, testcase.f));
    }

    #[test]
    fn test_trivial_noop_random1() {
        let testcase = TestCase::random_1();
        let man = testcase.man.clone();
        assert!(testcase.verify_against(&man, testcase.f));
    }

    // Test that the "trivial" testcase matches the "trivial" dimacs
    #[test]
    fn trivial_same_as_dimacs() {
        let testcase = TestCase::test_trivial();

        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string("examples/trivial.dimacs").expect("Failed to read dimacs file."),
        )
        .expect("Failed to parse dimacs file.");
        let (man_dimacs, bdd_dimacs) =
            DDManager::from_instance(&mut instance, None, Default::default()).unwrap();
        assert!(testcase.verify_against(&man_dimacs, bdd_dimacs));
    }

    #[test]
    fn truthtable_satcount_random1() {
        let testcase = TestCase::random_1();
        assert_eq!(testcase.man.sat_count(testcase.f), BigUint::from(132usize));
    }
}
