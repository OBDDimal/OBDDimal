use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::HashMap,
};

struct Bcdd {
    roots: Vec<isize>,
    varcount: usize,
    varorder: Vec<VarID>,
    nodes: HashMap<isize, (VarID, isize, isize)>,
}

impl DDManager {
    /// Reads a (multi-rooted) BDD from a .dddmp file.
    ///
    /// * `filename` - Name of the .dddmp file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// //let (man, bdds) = DDManager::load_from_dddmp_file("sandwich.dimacs.dddmp".to_string()).unwrap();
    /// ```
    pub fn load_from_dddmp_file(filename: String) -> Result<(DDManager, Vec<NodeID>), String> {
        let bcdd =
            Self::parse_bcdd_from_dddmp_file(File::open(filename).map_err(|e| e.to_string())?)?;
        let bdd = DDManager::default();
        Self::convert_bcdd_to_bdd(&bcdd, bdd)
    }

    /// Parses a BCDD from a .dddmp file.
    ///
    ///  * `file` - The file to be parsed
    ///
    fn parse_bcdd_from_dddmp_file(file: File) -> Result<Bcdd, String> {
        let lines = &mut BufReader::new(file)
            .lines()
            .map_while(Result::ok)
            .peekable();
        // Parse header:
        let header = lines
            .take_while(|line| line.trim() != ".nodes")
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut line = line.split_whitespace().map(str::to_string);
                (line.next().unwrap(), line.collect::<Vec<String>>())
            })
            .collect::<HashMap<String, Vec<String>>>();

        // Check dddmp version
        if let Some(version) = header.get(".ver") {
            if version.len() != 1 || version[0] != "DDDMP-2.0" {
                return Err("DDDMP file version not supported!".to_string());
            }
        } else {
            return Err("DDDMP file version missing!".to_string());
        };

        let nodecount = {
            let Some(value) = header.get(".nnodes") else {
                return Err(".nnodes missing!".to_string());
            };
            if value.len() != 1 {
                Err(".nnodes line invalid!".to_string())
            } else {
                Ok(value[0].parse::<usize>().map_err(|e| e.to_string())?)
            }
        }?;
        let varcount = {
            //TODO .nvars or .nsuppvars???
            let Some(value) = header.get(".nvars") else {
                return Err(".nvars missing!".to_string());
            };
            if value.len() != 1 {
                Err(".nvars line invalid!".to_string())
            } else {
                Ok(value[0].parse::<usize>().map_err(|e| e.to_string())?)
            }
        }?;
        let varorder = {
            let Some(order) = header.get(".permids") else {
                return Err(".permids missing!".to_string());
            };

            if order.is_empty() {
                Err(".permids line invalid!".to_string())
            } else {
                order
                    .iter()
                    .map(|id| Ok(VarID(id.parse::<usize>().map_err(|e| e.to_string())?)))
                    .try_collect::<Vec<VarID>>()
            }
        }?;
        let roots = {
            let Some(roots) = header.get(".rootids") else {
                return Err(".rootids missing!".to_string());
            };
            if roots.is_empty() {
                Err(".rootids line invalid!".to_string())
            } else {
                roots
                    .iter()
                    .map(|r| r.parse::<isize>().map_err(|e| e.to_string()))
                    .try_collect::<Vec<isize>>()
            }
        }?;

        Ok(Bcdd {
            roots,
            varcount,
            varorder,
            nodes: Self::parse_bcdd_nodelist(lines, nodecount)?,
        })
    }

    /// Parses the nodelist of a BCDD from a .dddmp file.
    ///
    ///  * `lines` - An iterator over the lines of the file, the header including the **.nodes** mark should already be consumed
    ///  * `expected_nodecount` - The number of nodes that is expected to be parsed (used for sanity checks)
    ///
    fn parse_bcdd_nodelist<I>(
        lines: &mut std::iter::Peekable<I>,
        expected_nodecount: usize,
    ) -> Result<HashMap<isize, (VarID, isize, isize)>, String>
    where
        I: std::iter::Iterator<Item = String>,
    {
        if lines.peek().is_none() {
            return Err("Node list missing in dddmp file!".to_string());
        }
        let nodes = lines
            .take_while(|line| line.trim() != ".end")
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let line: Vec<_> = line.split_whitespace().map(str::to_string).collect();
                if line.len() != 5 {
                    Err("Node list contains unexpected line!".to_string())
                } else {
                    Ok((
                        line[0].parse::<isize>().map_err(|e| e.to_string())?,
                        (
                            VarID(line[2].parse::<usize>().map_err(|e| e.to_string())?),
                            line[3].parse::<isize>().map_err(|e| e.to_string())?,
                            line[4].parse::<isize>().map_err(|e| e.to_string())?,
                        ),
                    ))
                }
            })
            .try_collect::<HashMap<isize, (VarID, isize, isize)>>()?;
        if nodes.len() != expected_nodecount {
            Err("Node list ended unexpectedly!".to_string())
        } else {
            Ok(nodes)
        }
    }

    /// Converts a BCDD to a normal BDD
    ///
    /// * `bcdd` - The BCDD to be converted
    /// * `bdd` - The DDManager the BDD is about to be stored in
    ///
    fn convert_bcdd_to_bdd(
        bcdd: &Bcdd,
        bdd: DDManager,
    ) -> Result<(DDManager, Vec<NodeID>), String> {
        todo!();
    }
}
