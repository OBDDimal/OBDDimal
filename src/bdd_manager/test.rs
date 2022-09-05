use crate::bdd_node::NodeID;

use super::DDManager;

impl DDManager {
    #[allow(dead_code)]
    fn verify(&self, f: NodeID, trues: &[u32]) -> bool {
        let mut values: Vec<bool> = vec![false; self.var2nodes.len() + 1];

        for x in trues {
            let x: usize = *x as usize;

            if x < values.len() {
                values[x] = true;
            } else {
                values[x] = false;
            }
        }

        let mut node_id = f;

        while node_id.0 >= 2 {
            let node = &self.nodes.get(&node_id).unwrap();

            if values[node.var.0 as usize] {
                node_id = node.high;
            } else {
                node_id = node.low;
            }
        }

        node_id.0 == 1
    }
}

#[cfg(test)]
pub mod tests {
    use std::fs;

    use crate::{
        bdd_manager::{ONE, ZERO},
        bdd_node::{NodeID, VarID},
    };

    use num_bigint::BigUint;
    use rustc_hash::FxHashSet as HashSet;

    /// A manually constructed BDD plus truth table, allowing verification of
    /// any other BDD agains it for testing if it represents the same function.
    pub struct TestCase {
        ones: HashSet<Vec<u32>>,
        pub man: DDManager,
        pub f: NodeID,
        pub nr_variables: u32,
    }

    impl TestCase {
        /// Construct testcase matching the "trivial" dimacs example
        /// (a + b + ~c) (a + ~b + c) (~a + ~b + c)
        /// ~a~b~c + ~abc + a~b~c + a~bc + abc
        /// ~b~c + bc + ac
        pub fn test_trivial() -> TestCase {
            let ones = HashSet::from_iter([vec![], vec![2, 3], vec![1], vec![1, 3], vec![1, 2, 3]]);
            let mut man = DDManager::default();

            let mut f = ZERO.id;

            let nr_variables = 3;

            for clause in ones.iter() {
                let mut c = ONE.id;
                for var in 1..nr_variables + 1 {
                    let v = if clause.contains(&var) {
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
            let mut clauses: HashSet<Vec<u32>> = HashSet::default();

            for line in table {
                let mut clause = Vec::new();
                for (var, value) in line.iter().enumerate() {
                    if *value != 0 {
                        clause.push(var as u32 + 1)
                    }
                }

                clauses.insert(clause);
            }

            let mut man = DDManager::default();
            let mut f = ZERO.id;

            for clause in clauses.iter() {
                let mut c = ONE.id;
                for var in 1..N as u32 + 1 {
                    let v = if clause.contains(&var) {
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
                nr_variables: N as u32,
            }
        }

        /// Test if a function in some other BDD matches this testcase
        #[must_use]
        pub fn verify_against(&self, other_man: &DDManager, other_f: NodeID) -> bool {
            for trues in self.ones.iter() {
                if !other_man.verify(other_f, trues) {
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

    use crate::{bdd_manager::DDManager, dimacs};

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

        let mut instance = dimacs::parse_dimacs("examples/trivial.dimacs");
        let (man_dimacs, bdd_dimacs) =
            DDManager::from_instance(&mut instance, None, false).unwrap();
        assert!(testcase.verify_against(&man_dimacs, bdd_dimacs));
    }

    #[test]
    fn truthtable_satcount_random1() {
        let testcase = TestCase::random_1();
        assert_eq!(testcase.man.sat_count(testcase.f), BigUint::from(132u32));
    }
}
