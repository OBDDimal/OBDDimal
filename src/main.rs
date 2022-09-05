use obddimal::{bdd_manager::options::Options, bdd_manager::DDManager, dimacs, static_ordering};

fn main() {
    env_logger::init();

    // let mut instance = dimacs::parse_dimacs("examples/cerf.dimacs");
    let mut instance = dimacs::parse_dimacs("examples/sandwich.dimacs");
    // let mut instance = dimacs::parse_dimacs("examples/trivial.dimacs");
    // let mut instance = dimacs::parse_dimacs("examples/berkeleydb.dimacs");
    // let mut instance = dimacs::parse_dimacs("examples/busybox.dimacs");

    let order = Some(static_ordering::force(&instance));

    let (man, bdd) = DDManager::from_instance(
        &mut instance,
        order,
        Options::default().with_progressbars().with_dvo(),
    )
    .unwrap();

    println!("Done! BDD has {} nodes.", man.count_active(bdd));

    println!("Starting #SAT");
    println!("{:?}", man.sat_count(bdd));
}

#[cfg(test)]
mod tests {

    use super::*;
    use num_bigint::BigUint;

    fn build_verify_ssat(filepath: &str, target: &[u8]) {
        let expected = BigUint::parse_bytes(target, 10).unwrap();

        let mut instance = dimacs::parse_dimacs(filepath);
        let (man, bdd) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
    }

    #[test]
    fn sandwich_ssat() {
        build_verify_ssat("examples/sandwich.dimacs", b"2808")
    }

    #[test]
    fn berkeleydb_ssat() {
        build_verify_ssat("examples/berkeleydb.dimacs", b"4080389785")
    }

    #[test]
    fn trivial_ssat() {
        build_verify_ssat("examples/trivial.dimacs", b"5")
    }

    #[test]
    #[ignore]
    fn busybox_ssat() {
        build_verify_ssat("examples/busybox.dimacs", b"FAIL")
    }
}
