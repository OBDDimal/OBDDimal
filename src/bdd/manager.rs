use std::{fmt::Display, sync::Arc, sync::Mutex, write};

use crate::bdd_ds::InputFormat;
use crate::bdd_ds::UniqueKey;
use crate::parser::{DataFormatError, ParserSettings};
use crate::{bdd_ds::Bdd, input::static_ordering::StaticOrdering};

use super::bdd_graph::NodeType;
use fnv::FnvHashMap;

#[derive(Debug)]
pub enum NoBddError {
    NoBddCreated,
}

impl Display for NoBddError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NoBddError::NoBddCreated => write!(
                f,
                "There is no BDD for the current manager. Try to add a BDD first."
            ),
        }
    }
}

pub trait Manager {
    fn get_bdd(&self) -> &Option<Bdd>;
    fn get_sat_count(&self) -> &Option<u64>;
    fn get_node_count(&self) -> &Option<u64>;
    fn get_bdd_mut(&mut self) -> &mut Option<Bdd>;
    fn get_sat_count_mut(&mut self) -> &mut Option<u64>;
    fn get_node_count_mut(&mut self) -> &mut Option<u64>;

    /// Creates an empty 'Manager' struct.
    fn new() -> Self;

    /// Creates a new BDD from a given input.
    /// `format` is the given format of the input.
    /// `settings` describe how the parser should interpret the input.
    /// Returns a `BddManager` or a `DataFormatError`.
    fn from_format(
        cnf: &str,
        format: InputFormat,
        settings: ParserSettings,
        static_ordering: StaticOrdering,
    ) -> Result<Self, DataFormatError>
    where
        Self: Sized;

    /// Adds a given `Bdd` to `Manager` and resets the memoization
    /// of the sat_count and node_count.
    fn add_bdd(&mut self, bdd: Bdd) -> &mut Self {
        *self.get_bdd_mut() = Some(bdd);
        *self.get_node_count_mut() = None;
        *self.get_sat_count_mut() = None;
        self
    }

    /// Deserializes a previously serialized `Bdd` onto the `Manager`.
    fn deserialize_bdd(&mut self, input: &str) -> &mut Self {
        self.add_bdd(Bdd::deserialize(String::from(input)))
    }

    /// Serializes the current hold `Bdd` to a `String`.
    fn serialize_bdd(&self) -> Result<String, NoBddError> {
        let bdd = if let Some(x) = &self.get_bdd() {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        Ok(bdd.serialize())
    }

    /// Counts the nodes of the currently hold `Bdd` and
    /// returns the result `Result<u64, NoBddError>`.
    fn node_count(&mut self) -> Result<u64, NoBddError> {
        let bdd = if let Some(x) = &self.get_bdd() {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        match self.get_node_count() {
            Some(nc) => Ok(*nc),
            None => {
                let nc = bdd.nodecount();
                *self.get_node_count_mut() = Some(nc);
                Ok(nc)
            }
        }
    }

    /// Returns `Ok(true)` is the given `Bdd` represents a function
    /// which is satisfiable.
    fn satisfiable(&self) -> Result<bool, NoBddError> {
        let bdd = if let Some(x) = &self.get_bdd() {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        Ok(bdd.satisfiable())
    }

    /// Returns the number of inputs that satisfy the function the current `Bdd`
    /// is representing.
    fn sat_count(&mut self) -> Result<u64, NoBddError> {
        let bdd = if let Some(x) = &self.get_bdd() {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        match self.get_sat_count() {
            Some(x) => Ok(*x),
            None => {
                let sc = bdd.satcount();
                *self.get_sat_count_mut() = Some(sc);
                Ok(sc)
            }
        }
    }
}

#[derive(Debug)]
pub struct BddManager {
    bdd: Option<Bdd>,
    sat_count: Option<u64>,
    node_count: Option<u64>,
}

impl Manager for BddManager {
    fn new() -> Self {
        BddManager {
            bdd: None,
            sat_count: None,
            node_count: None,
        }
    }

    /// Creates a new `BddManager` by doing the build_process in a sequential fashion.
    /// cnf: is a string containing the conjunctive normal form in DIMACS CNF.
    /// format: is the format of the input, currently only DIMACS CNF is supported.
    /// settings: are the settings for the input parser (ParserSettings::Default()).
    /// static_ordering: is the heuristic used for static ordering, currently only NONE and FORCE.
    fn from_format(
        cnf: &str,
        format: InputFormat,
        settings: ParserSettings,
        static_ordering: StaticOrdering,
    ) -> Result<Self, DataFormatError> {
        let bdd = Bdd::from_format(cnf, format, settings, static_ordering)?;
        Ok(BddManager {
            bdd: Some(bdd),
            sat_count: None,
            node_count: None,
        })
    }

    fn get_bdd_mut(&mut self) -> &mut Option<Bdd> {
        &mut self.bdd
    }

    fn get_sat_count_mut(&mut self) -> &mut Option<u64> {
        &mut self.sat_count
    }

    fn get_node_count_mut(&mut self) -> &mut Option<u64> {
        &mut self.node_count
    }

    fn get_bdd(&self) -> &Option<Bdd> {
        &self.bdd
    }

    fn get_sat_count(&self) -> &Option<u64> {
        &self.sat_count
    }

    fn get_node_count(&self) -> &Option<u64> {
        &self.node_count
    }
}

#[derive(Debug)]
pub struct BddParaManager {
    bdd: Option<Bdd>,
    sat_count: Option<u64>,
    node_count: Option<u64>,
    unique_table: Option<fnv::FnvHashMap<UniqueKey, Arc<NodeType>>>,
}

impl Manager for BddParaManager {
    fn new() -> Self {
        Self {
            bdd: None,
            sat_count: None,
            node_count: None,
            unique_table: None,
        }
    }

    /// Creates a new `BddParaManager` by doing the build_process in a parallel fashion.
    /// cnf: is a string containing the conjunctive normal form in DIMACS CNF.
    /// format: is the format of the input, currently only DIMACS CNF is supported.
    /// settings: are the settings for the input parser (ParserSettings::Default()).
    /// static_ordering: is the heuristic used for static ordering, currently only NONE and FORCE.
    fn from_format(
        cnf: &str,
        format: InputFormat,
        settings: ParserSettings,
        static_ordering: StaticOrdering,
    ) -> Result<Self, DataFormatError> {
        let unique_table = Arc::new(Mutex::new(FnvHashMap::default()));
        let bdd = Bdd::from_format_para(cnf, format, settings, static_ordering, unique_table)?;

        Ok(BddParaManager {
            bdd: Some(bdd),
            sat_count: None,
            node_count: None,
            unique_table: None,
        })
    }

    fn get_bdd_mut(&mut self) -> &mut Option<Bdd> {
        &mut self.bdd
    }

    fn get_sat_count_mut(&mut self) -> &mut Option<u64> {
        &mut self.sat_count
    }

    fn get_node_count_mut(&mut self) -> &mut Option<u64> {
        &mut self.node_count
    }

    fn get_bdd(&self) -> &Option<Bdd> {
        &self.bdd
    }

    fn get_sat_count(&self) -> &Option<u64> {
        &self.sat_count
    }

    fn get_node_count(&self) -> &Option<u64> {
        &self.node_count
    }
}
