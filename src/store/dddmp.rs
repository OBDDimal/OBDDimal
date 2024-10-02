use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{
    core::{
        bdd_manager::DDManager,
        bdd_node::{NodeID, VarID},
    },
    misc::hash_select::{HashMap, HashSet},
};

type NodeList = HashMap<isize, (VarID, isize, isize)>;

/// Stores a BCDD (quiet similar to its representation in the .dddmp file)
///
/// * `roots` - The root nodes of the BCDD (may be inverted)
/// * `varorder` - The order of the variables of the BCDD
/// * `nodes` - Maps Node IDs to a tuple containing their Variable and the IDs of the high and low successors (negative ID if inverted edge)
/// * `terminal_ids` - The ids of the terminal node(s) of the BCDD, first one is the high node (always there), the second one the low node if it exists
/// * `add` - Signals that the add flag was set in the file (which means that no complemented edges exist in it)
///
struct Bcdd {
    roots: Vec<isize>,
    varorder: Vec<usize>,
    nodes: NodeList,
    terminal_ids: (isize, Option<isize>),
    add: bool,
}

/// Represents a parent node, containing the ID of the node.
/// For root nodes, a ParentNode::Root() is stored to express their special status.
#[derive(Eq, Hash, PartialEq)]
enum ParentNode {
    Normal(isize),
    Root(),
}

impl DDManager {
    /// Reads a (multi-rooted) BDD from a .dddmp file.
    ///
    /// * `filename` - Name of the .dddmp file.
    ///
    /// ```
    /// # use obddimal::core::bdd_manager::DDManager;
    /// let (man, bdds) =
    /// DDManager::load_from_dddmp_file("examples/sandwich.dimacs.dddmp".to_string()).unwrap();
    /// ```
    pub fn load_from_dddmp_file(filename: String) -> Result<(DDManager, Vec<NodeID>), String> {
        let bcdd =
            Self::parse_bcdd_from_dddmp_file(File::open(filename).map_err(|e| e.to_string())?)?;
        Ok(Self::convert_bcdd_to_bdd(&bcdd))
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

        // Check for .add flag
        let add = header.contains_key(".add");

        // Verify file integrity
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
            let Some(value) = header.get(".nsuppvars") else {
                return Err(".nsuppvars missing!".to_string());
            };
            if value.len() != 1 {
                Err(".nsuppvars line invalid!".to_string())
            } else {
                Ok(value[0].parse::<usize>().map_err(|e| e.to_string())?)
            }
        }?;
        let varcount_with_free = {
            let Some(value) = header.get(".nvars") else {
                return Err(".nvars missing!".to_string());
            };
            if value.len() != 1 {
                Err(".nvars line invalid!".to_string())
            } else {
                Ok(value[0].parse::<usize>().map_err(|e| e.to_string())?)
            }
        }?;

        // Parse variable ordering
        let varorder: Vec<usize> = {
            let Some(ids) = header.get(".ids") else {
                return Err(".ids missing!".to_string());
            };
            let ids = ids
                .iter()
                .map(|s| s.parse::<usize>().map_err(|e| e.to_string()))
                .try_collect::<Vec<usize>>()?;
            let Some(permids) = header.get(".permids") else {
                return Err(".permids missing!".to_string());
            };
            let permids = permids
                .iter()
                .map(|s| s.parse::<usize>().map_err(|e| e.to_string()))
                .try_collect::<Vec<usize>>()?;
            if ids.len() != varcount {
                return Err(".ids line invalid!".to_string());
            };
            if permids.len() != varcount {
                return Err(".permids line invalid!".to_string());
            };

            let mut order: Vec<usize> = vec![0; varcount_with_free + 1usize];
            let mut free_vars: HashSet<usize> = (1..(varcount_with_free + 1usize)).collect();
            let mut free_levels: HashSet<usize> = (1..(varcount_with_free + 1usize)).collect();
            permids.iter().enumerate().for_each(|(i, permid)| {
                let var_id = ids[i] + 1usize;
                let level = *permid + 1;
                order[var_id] = level;
                free_vars.remove(&var_id);
                free_levels.remove(&level);
            });
            order[0] = varcount_with_free + 1usize;

            // Add levels to free variables
            free_vars
                .iter()
                .zip(free_levels.iter())
                .for_each(|(var_id, level)| {
                    order[*var_id] = *level;
                });

            Ok::<Vec<usize>, String>(order)
        }?;

        // Parse root nodes
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

        // Parse node list
        let (nodes, terminal_ids) = Self::parse_bcdd_nodelist(lines, nodecount, add)?;

        // Check if root nodes are valid:
        for r in roots.iter() {
            if !nodes.contains_key(&r.abs()) {
                return Err("Root node not existant in BDD!".to_string());
            }
        }

        Ok(Bcdd {
            roots,
            varorder,
            nodes,
            terminal_ids,
            add,
        })
    }

    /// Parses the nodelist of a BCDD from a .dddmp file.
    ///
    ///  * `lines` - An iterator over the lines of the file, the header including the **.nodes** mark should already be consumed
    ///  * `expected_nodecount` - The number of nodes that is expected to be parsed (used for sanity checks)
    ///  * `add` - Whether the add flag was set in the file (which means that no complemented edges exist in it)
    ///
    fn parse_bcdd_nodelist<I>(
        lines: &mut std::iter::Peekable<I>,
        expected_nodecount: usize,
        add: bool,
    ) -> Result<(NodeList, (isize, Option<isize>)), String>
    where
        I: std::iter::Iterator<Item = String>,
    {
        if lines.peek().is_none() {
            return Err("Node list missing in dddmp file!".to_string());
        }
        let mut terminal_ids = (None, None);
        let nodes = lines
            .take_while(|line| line.trim() != ".end")
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let line: Vec<_> = line.split_whitespace().map(str::to_string).collect();
                if line.len() != 5 {
                    Err("Node list contains unexpected line!".to_string())
                } else {
                    let id = line[0].parse::<isize>().map_err(|e| e.to_string())?;
                    let high = line[3].parse::<isize>().map_err(|e| e.to_string())?;
                    let low = line[4].parse::<isize>().map_err(|e| e.to_string())?;
                    let var_id = if line[1] == "T" {
                        if !add {
                            terminal_ids = (Some(id), None);
                        } else {
                            terminal_ids =
                                match line[2].parse::<isize>().map_err(|e| e.to_string())? {
                                    0 => (terminal_ids.0, Some(id)),
                                    1 => (Some(id), terminal_ids.1),
                                    _ => return Err("Terminal Node not supported".to_string()),
                                }
                        }
                        VarID(0)
                    } else {
                        VarID(line[1].parse::<usize>().map_err(|e| e.to_string())? + 1usize)
                    };
                    Ok((id, (var_id, high, low)))
                }
            })
            .try_collect::<NodeList>()?;

        if nodes.len() != expected_nodecount {
            Err("Node list ended unexpectedly!".to_string())
        } else if terminal_ids.0.is_none() || (add && terminal_ids.1.is_none()) {
            Err("Terminal node missing!".to_string())
        } else {
            let terminal_ids = if !add {
                (terminal_ids.0.unwrap(), None)
            } else {
                (terminal_ids.0.unwrap(), terminal_ids.1)
            };

            Ok((nodes, terminal_ids))
        }
    }

    /// Converts a BCDD to a normal BDD
    ///
    /// * `bcdd` - The BCDD to be converted
    ///
    fn convert_bcdd_to_bdd(bcdd: &Bcdd) -> (DDManager, Vec<NodeID>) {
        let bdd_nodes = if !bcdd.add {
            Self::convert_bcdd_to_bdd_nodes(
                Self::create_bcdd_node_parent_information(bcdd),
                Self::create_bcdd_layer_to_nodes(bcdd),
                bcdd,
            )
        } else {
            bcdd.nodes.clone()
        };

        // Convert NodeIDs:
        let convert_node_id = |i: &isize| NodeID(*i as usize);
        let bdd_nodes = bdd_nodes
            .iter()
            .map(|(n, (v, c1, c2))| {
                (
                    convert_node_id(n),
                    (*v, convert_node_id(c1), convert_node_id(c2)),
                )
            })
            .collect::<HashMap<NodeID, (VarID, NodeID, NodeID)>>();

        let roots = bcdd
            .roots
            .iter()
            .map(convert_node_id)
            .collect::<Vec<NodeID>>();

        let terminals = (
            convert_node_id(&bcdd.terminal_ids.0),
            convert_node_id(&if !bcdd.add {
                -bcdd.terminal_ids.0
            } else {
                bcdd.terminal_ids.1.unwrap()
            }),
        );

        DDManager::default().load_bdd_from_nodelist(
            bdd_nodes,
            bcdd.varorder.clone(),
            roots,
            terminals,
        )
    }

    /// Creates a HashMap containing information about the parents of each node and the edges
    /// connecting them.
    ///
    /// * `bcdd` - The BCDD containing the nodes
    ///
    fn create_bcdd_node_parent_information(
        bcdd: &Bcdd,
    ) -> HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)> {
        debug_assert!(!bcdd.add);

        let mut node_parent_information = bcdd
            .nodes
            .keys()
            .map(|k| (*k, (HashSet::default(), HashSet::default())))
            .collect::<HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)>>();
        bcdd.nodes
            .iter()
            .flat_map(|(p, (_, c1, c2))| [(*p, *c1), (*p, *c2)])
            .filter(|(p, _)| *p != bcdd.terminal_ids.0)
            .for_each(|(p, c)| {
                let info = node_parent_information.get_mut(&c.abs()).unwrap();
                if c < 0 {
                    info.1.insert(ParentNode::Normal(p));
                } else {
                    info.0.insert(ParentNode::Normal(p));
                }
            });
        bcdd.roots.iter().for_each(|r| {
            let info = node_parent_information.get_mut(&r.abs()).unwrap();
            if *r < 0 {
                info.1.insert(ParentNode::Root());
            } else {
                info.0.insert(ParentNode::Root());
            }
        });
        node_parent_information
    }

    /// Creates a HashMap containing the node IDs for each layer.
    ///
    /// * `bcdd` - The BCDD containing the nodes
    ///
    fn create_bcdd_layer_to_nodes(bcdd: &Bcdd) -> HashMap<usize, HashSet<isize>> {
        debug_assert!(!bcdd.add);

        bcdd.nodes
            .iter()
            .filter(|(n, _)| **n != bcdd.terminal_ids.0)
            .map(|(n, (v, _, _))| (bcdd.varorder[v.0], n))
            .fold(HashMap::default(), |mut layer_to_nodes, (l, n)| {
                if let Some(nodes) = layer_to_nodes.get_mut(&l) {
                    nodes.insert(*n);
                } else {
                    let mut nodes = HashSet::default();
                    nodes.insert(*n);
                    layer_to_nodes.insert(l, nodes);
                }
                layer_to_nodes
            })
    }

    /// Creates a HashTable containing the nodes of a BDD representing the same function as the
    /// given BCDD.
    ///
    /// * `node_parent_information` - HashMap storing the parents of a node.
    /// * `layer_to_nodes` - HashMap mapping the layers of the bcdd to its nodes
    /// * `bcdd` - The BCDD which is going to be converted to a BDD.
    fn convert_bcdd_to_bdd_nodes(
        mut node_parent_information: HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)>,
        layer_to_nodes: HashMap<usize, HashSet<isize>>,
        bcdd: &Bcdd,
    ) -> NodeList {
        debug_assert!(!bcdd.add);

        let mut bdd_nodes = HashMap::default();

        let mut layers = bcdd.varorder.clone();
        layers.sort();
        layers
            .iter()
            .filter(|layer| layer_to_nodes.contains_key(layer))
            .flat_map(|layer| layer_to_nodes.get(layer).unwrap())
            .filter(|node_id| **node_id != bcdd.terminal_ids.0)
            .for_each(|node_id| {
                let node_info = *bcdd.nodes.get(node_id).unwrap();
                let parents_info = node_parent_information.get(node_id).unwrap();
                let node_id = *node_id;
                let mut normal_needed = false;
                if !parents_info.0.is_empty() {
                    // If node is required uninverted, add node, childs stay as they are:
                    bdd_nodes.insert(node_id, node_info);
                    normal_needed = true;
                }
                if !parents_info.1.is_empty() {
                    // If node is required inverted, add new node with inverted childs:
                    let (v, c1, c2) = node_info;
                    let mut update_child = |c: isize| {
                        let p = ParentNode::Normal(node_id);
                        let (ref mut p_normal, ref mut p_inverted) =
                            node_parent_information.get_mut(&c.abs()).unwrap();
                        let (from, to) = if c < 0 {
                            (p_inverted, p_normal)
                        } else {
                            (p_normal, p_inverted)
                        };
                        if !normal_needed {
                            // Only remove if only the inverted version is required
                            from.remove(&p);
                        }
                        to.insert(p);
                    };
                    update_child(c1);
                    update_child(c2);
                    bdd_nodes.insert(-node_id, (v, -c1, -c2));
                }
            });

        // Add 0 and 1 nodes:
        bdd_nodes.insert(bcdd.terminal_ids.0, (VarID(0), 1, 1));
        bdd_nodes.insert(-bcdd.terminal_ids.0, (VarID(0), 0, 0));

        bdd_nodes
    }
}

#[cfg(test)]
mod test {
    use malachite::Natural;

    use crate::core::bdd_manager::DDManager;

    #[test]
    fn dddmp_file_read_soletta() {
        dddmp_file_read_compare_with_add(
            "examples/soletta.dddmp".to_string(),
            "examples/soletta_nce.dddmp".to_string(),
            None,
        )
    }

    #[test]
    fn dddmp_file_read_busybox() {
        dddmp_file_read_compare_with_add(
            "examples/busybox.dddmp".to_string(),
            "examples/busybox_nce.dddmp".to_string(),
            None,
        )
    }

    #[test]
    fn dddmp_file_read_uclibc() {
        dddmp_file_read_compare_with_add(
            "examples/uclibc.dddmp".to_string(),
            "examples/uclibc_nce.dddmp".to_string(),
            None,
        )
    }

    #[test]
    fn dddmp_file_read_fiasco() {
        dddmp_file_read_compare_with_add(
            "examples/fiasco.dddmp".to_string(),
            "examples/fiasco_nce.dddmp".to_string(),
            None,
        )
    }

    #[inline]
    fn dddmp_file_read_compare_with_add(
        dddmp_file: String,
        dddmp_add_file: String,
        ssat_expected: Option<Natural>,
    ) {
        let (man_complemented, bdds_complemented) =
            DDManager::load_from_dddmp_file(dddmp_file).unwrap();
        let root_complemented = bdds_complemented[0];
        let ssat_complemented = man_complemented.sat_count(root_complemented);

        let (man_non_complemented, bdds_non_complemented) =
            DDManager::load_from_dddmp_file(dddmp_add_file).unwrap();
        let root_non_complemented = bdds_non_complemented[0];
        let ssat_non_complemented = man_non_complemented.sat_count(root_non_complemented);

        if ssat_expected.is_some() {
            assert_eq!(&ssat_complemented, ssat_expected.as_ref().unwrap());
            assert_eq!(&ssat_non_complemented, ssat_expected.as_ref().unwrap());
        } else {
            assert_eq!(ssat_non_complemented, ssat_complemented);
        }
    }

    #[test]
    fn dddmp_file_read_sandwich() {
        dddmp_file_read_verify_ssat(
            "examples/sandwich.dimacs.dddmp".to_string(),
            2808usize.into(),
        )
    }

    #[test]
    fn dddmp_file_read_jhipster() {
        dddmp_file_read_verify_ssat(
            "examples/JHipster.dimacs.dddmp".to_string(),
            26256usize.into(),
        )
    }

    #[test]
    fn dddmp_file_read_berkeleydb() {
        dddmp_file_read_verify_ssat(
            "examples/berkeleydb.dimacs.dddmp".to_string(),
            4080389785u64.into(),
        )
    }

    #[inline]
    fn dddmp_file_read_verify_ssat(dddmp_file: String, ssat_expected: Natural) {
        let (man, bdds) = DDManager::load_from_dddmp_file(dddmp_file).unwrap();
        let root = bdds[0];
        assert_eq!(man.sat_count(root), ssat_expected);
    }

    #[test]
    fn dddmp_file_read_invalid() {
        let result = DDManager::load_from_dddmp_file("examples/invalid.dddmp".to_string());
        assert!(result.is_err());
    }
}
