#[cfg(test)]
pub mod tests {
    use std::fs;

    use crate::{
        bdd_manager::{ONE, ZERO},
        bdd_node::{NodeID, VarID},
    };

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
    fn test_trivial_noop() {
        let testcase = TestCase::test_trivial();
        let man = testcase.man.clone();
        assert!(testcase.verify_against(&man, testcase.f));
    }

    // Test that the "trivial" testcase matches the "trivial" dimacs
    #[test]
    fn trivial_same_as_dimacs() {
        let testcase = TestCase::test_trivial();

        let mut instance = dimacs::parse_dimacs("examples/trivial.dimacs");
        let (man_dimacs, bdd_dimacs) = DDManager::from_instance(&mut instance, None).unwrap();
        assert!(testcase.verify_against(&man_dimacs, bdd_dimacs));
    }
}
