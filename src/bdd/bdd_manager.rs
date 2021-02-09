use crate::bdd_ds::Bdd;
use crate::bdd_ds::InputFormat;
use crate::parser::{DataFormatError, ParserSettings};

#[derive(Debug)]
pub enum NoBddError {
    NoBddCreated,
}
#[derive(Debug)]
pub struct BddManager {
    bdd: Option<Bdd>,
    sat_count: Option<u64>,
    node_count: Option<u64>,
}

impl BddManager {
    pub fn new() -> Self {
        BddManager {
            bdd: None,
            sat_count: None,
            node_count: None,
        }
    }

    /// Creates a new BDD from a given input.
    /// `format` is the given format of the input.
    /// `settings` describe how the parser should interpret the input.
    /// Returns a `BddManager` or a `DataFormatError`.
    pub fn new_from_format(
        cnf: &str,
        format: InputFormat,
        settings: ParserSettings,
    ) -> Result<Self, DataFormatError> {
        let bdd = Bdd::from_format(cnf, format, settings)?;
        Ok(BddManager {
            bdd: Some(bdd),
            sat_count: None,
            node_count: None,
        })
    }

    /// Creates a new BddManager out of a `BddManager` and a `Bdd`.
    pub fn add_bdd(mut self, bdd: Bdd) -> Self {
        self.bdd = Some(bdd);
        self.node_count = None;
        self.sat_count = None;
        self
    }

    pub fn deserialize_bdd(mut self, input: &str) -> Self {
        self.bdd = Some(Bdd::deserialize(String::from(input)));
        self.node_count = None;
        self.sat_count = None;
        self
    }

    pub fn serialize_bdd(&self) -> Result<String, NoBddError> {
        let bdd = if let Some(x) = &self.bdd {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        Ok(bdd.serialize())
    }

    /// Returns the number of nodes in the bdd as a `Result<u64, NoBddError>`.
    pub fn node_count(&mut self) -> Result<u64, NoBddError> {
        let bdd = if let Some(x) = &self.bdd {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        match self.node_count {
            Some(nc) => Ok(nc),
            None => {
                let nc = bdd.nodecount();
                self.node_count = Some(nc);
                Ok(nc)
            }
        }
    }

    /// Returns if the bdd is satisfiable as a `Result<bool, NoBddError>`.
    pub fn satisfiable(&self) -> Result<bool, NoBddError> {
        let bdd = if let Some(x) = &self.bdd {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        Ok(bdd.satisfiable())
    }

    /// Returns the number of ways the bdd is satisfiable as a `Result<u64, NoBddError>`.
    pub fn sat_count(&mut self) -> Result<u64, NoBddError> {
        let bdd = if let Some(x) = &self.bdd {
            x
        } else {
            return Err(NoBddError::NoBddCreated);
        };

        match self.sat_count {
            Some(x) => Ok(x),
            None => {
                let sc = bdd.satcount();
                self.sat_count = Some(sc);
                Ok(sc)
            }
        }
    }
}
