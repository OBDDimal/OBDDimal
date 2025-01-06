#![feature(iterator_try_collect)]
use std::fs;

use obddimal::{
    core::{bdd_manager::DDManager, dvo::dvo_schedules, options::Options},
    misc::static_ordering,
};

fn main() {
    env_logger::init();

    // let filepath = "examples/cerf.dimacs";
    // let filepath = "examples/sandwich.dimacs";
    // let filepath = "examples/trivial.dimacs";
    // let filepath = "examples/berkeleydb.dimacs";
    let filepath = "examples/busybox.dimacs";
    let mut instance = dimacs::parse_dimacs(
        &fs::read_to_string(filepath)
            .unwrap_or_else(|_| panic!("Failed to read dimacs file: {}", filepath)),
    )
    .unwrap_or_else(|_| panic!("Failed to parse dimacs file: {}", filepath));

    let order = Some(static_ordering::force(&instance));

    let dvo = dvo_schedules::AtThreshold {
        active_nodes_threshold: 50000,
        underlying_schedule: Box::new(
            dvo_schedules::AlwaysOnce {
                max_increase: Some(1000),
            }
            .into(),
        ),
    };

    let (man, bdd) = DDManager::from_instance(
        &mut instance,
        order,
        Options::default().with_progressbars().with_dvo(dvo.into()),
    )
    .unwrap();

    println!("Done! BDD has {} nodes.", man.count_active(bdd));

    println!("Starting #SAT");
    println!("{:?}", man.sat_count(bdd));
}

#[cfg(test)]
mod tests {

    use malachite::Natural;

    use super::*;

    fn build_verify_ssat(filepath: &str, target: &str) {
        let expected = target.parse::<Natural>().unwrap();

        let mut instance = dimacs::parse_dimacs(
            &fs::read_to_string(filepath)
                .unwrap_or_else(|_| panic!("Failed to read dimacs file: {}", filepath)),
        )
        .unwrap_or_else(|_| panic!("Failed to parse dimacs file: {}", filepath));
        let (man, bdd) = DDManager::from_instance(&mut instance, None, Default::default()).unwrap();

        assert_eq!(man.sat_count(bdd), expected);
    }

    #[test]
    fn sandwich_ssat() {
        build_verify_ssat("examples/sandwich.dimacs", "2808")
    }

    #[test]
    fn berkeleydb_ssat() {
        build_verify_ssat("examples/berkeleydb.dimacs", "4080389785")
    }

    #[test]
    fn trivial_ssat() {
        build_verify_ssat("examples/trivial.dimacs", "5")
    }

    #[test]
    #[ignore]
    fn busybox_ssat() {
        build_verify_ssat("examples/busybox.dimacs", "2061138519356781760670618805653750167349287991336595876373542198990734653489713239449032049664199494301454199336000050382457451123894821886472278234849758979132037884598159833615564800000000000000000000")
    }
}
