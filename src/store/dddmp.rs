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
/// * `varcount` - The number of variables in the BCDD
/// * `varorder` - The order of the variables of the BCDD
/// * `nodes` - Maps Node IDs to a tuple containing their Variable and the IDs of the high and low successors (negative ID if inverted edge)
///
struct Bcdd {
    roots: Vec<isize>,
    varorder: Vec<VarID>,
    nodes: NodeList,
    terminal_id: isize,
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
    /// //let (man, bdds) = DDManager::load_from_dddmp_file("sandwich.dimacs.dddmp".to_string()).unwrap();
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

        // Check dddmp version
        if let Some(version) = header.get(".ver") {
            if version.len() != 1 || version[0] != "DDDMP-2.0" {
                return Err("DDDMP file version not supported!".to_string());
            }
        } else {
            return Err("DDDMP file version missing!".to_string());
        };

        // To verify file integrity
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
        let varorder = {
            let Some(order) = header.get(".permids") else {
                return Err(".permids missing!".to_string());
            };

            if order.len() != varcount {
                Err(".permids line invalid!".to_string())
            } else {
                order
                    .iter()
                    .map(|id| Ok(VarID(id.parse::<usize>().map_err(|e| e.to_string())?)))
                    .try_collect::<Vec<VarID>>()
            }
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
        let (nodes, terminal_id) = Self::parse_bcdd_nodelist(lines, nodecount)?;

        Ok(Bcdd {
            roots,
            varorder,
            nodes,
            terminal_id,
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
    ) -> Result<(NodeList, isize), String>
    where
        I: std::iter::Iterator<Item = String>,
    {
        if lines.peek().is_none() {
            return Err("Node list missing in dddmp file!".to_string());
        }
        let mut terminal_id = None;
        let nodes = lines
            .take_while(|line| line.trim() != ".end")
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let line: Vec<_> = line.split_whitespace().map(str::to_string).collect();
                if line.len() != 5 {
                    Err("Node list contains unexpected line!".to_string())
                } else {
                    let id = line[0].parse::<isize>().map_err(|e| e.to_string())?;
                    if line[1] == "T" {
                        terminal_id = Some(id);
                    }
                    Ok((
                        id,
                        (
                            VarID(line[2].parse::<usize>().map_err(|e| e.to_string())?),
                            line[3].parse::<isize>().map_err(|e| e.to_string())?,
                            line[4].parse::<isize>().map_err(|e| e.to_string())?,
                        ),
                    ))
                }
            })
            .try_collect::<NodeList>()?;
        if nodes.len() != expected_nodecount {
            Err("Node list ended unexpectedly!".to_string())
        } else if terminal_id.is_none() {
            Err("Terminal node missing!".to_string())
        } else {
            Ok((nodes, terminal_id.unwrap()))
        }
    }

    /// Converts a BCDD to a normal BDD
    ///
    /// * `bcdd` - The BCDD to be converted
    ///
    fn convert_bcdd_to_bdd(bcdd: &Bcdd) -> (DDManager, Vec<NodeID>) {
        let bdd_nodes = Self::convert_bcdd_to_bdd_nodes(
            Self::create_bcdd_node_parent_information(bcdd),
            Self::create_bcdd_layer_to_nodes(bcdd),
            bcdd,
        );

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
            convert_node_id(&bcdd.terminal_id),
            convert_node_id(&-bcdd.terminal_id),
        );

        let varorder = Vec::default(); // TODO varorder
        DDManager::default().load_bdd_from_nodelist(bdd_nodes, varorder, roots, terminals)
    }

    /// Creates a HashMap containing information about the parents of each node and the edges
    /// connecting them.
    ///
    /// * `bcdd` - The BCDD containing the nodes
    ///
    fn create_bcdd_node_parent_information(
        bcdd: &Bcdd,
    ) -> HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)> {
        let mut node_parent_information = bcdd
            .nodes
            .keys()
            .map(|k| (*k, (HashSet::default(), HashSet::default())))
            .collect::<HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)>>();
        bcdd.nodes
            .iter()
            .flat_map(|(p, (_, c1, c2))| [(*p, *c1), (*p, *c2)])
            .filter(|(p, _)| *p != bcdd.terminal_id)
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
    fn create_bcdd_layer_to_nodes(bcdd: &Bcdd) -> HashMap<usize, Vec<isize>> {
        let mut var_to_nodes = bcdd
            .varorder
            .iter()
            .map(|v| (*v, Vec::default()))
            .collect::<HashMap<VarID, Vec<isize>>>();
        bcdd.nodes
            .iter()
            .filter(|(n, _)| **n != bcdd.terminal_id)
            .map(|(n, (v, _, _))| (v, n))
            .for_each(|(v, n)| {
                let nodes = var_to_nodes.get_mut(v).unwrap();
                nodes.push(*n);
            });
        bcdd.varorder
            .iter()
            .enumerate()
            .map(|(l, v)| (l, var_to_nodes.get(v).unwrap().clone()))
            .collect()
    }

    /// Creates a HashTable containing the nodes of a BDD representing the same function as the
    /// given BCDD.
    ///
    /// * `node_parent_information` - HashMap storing the parents of a node.
    /// * `layer_to_nodes` - HashMap mapping the layers of the bcdd to its nodes
    /// * `bcdd` - The BCDD which is going to be converted to a BDD.
    fn convert_bcdd_to_bdd_nodes(
        mut node_parent_information: HashMap<isize, (HashSet<ParentNode>, HashSet<ParentNode>)>,
        layer_to_nodes: HashMap<usize, Vec<isize>>,
        bcdd: &Bcdd,
    ) -> NodeList {
        let mut bdd_nodes = HashMap::default();
        for l in layer_to_nodes
            .keys()
            .collect::<std::collections::BinaryHeap<_>>() // sort…
            .iter()
        {
            layer_to_nodes.get(l).unwrap().iter().for_each(|node| {
                let node_info = *bcdd.nodes.get(node).unwrap();
                let parents_info = node_parent_information.get(node).unwrap();
                let node = *node;
                let mut normal_needed = false;
                if !parents_info.0.is_empty() {
                    // If node is required uninverted, add node, childs stay as they are:
                    bdd_nodes.insert(node, node_info);
                    normal_needed = true;
                }
                if !parents_info.1.is_empty() {
                    // If node is required inverted, add new node with inverted childs:
                    let (v, c1, c2) = node_info;
                    let mut update_child = |c: isize| {
                        let p = ParentNode::Normal(node);
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
                    bdd_nodes.insert(-node, (v, -c1, -c2));
                }
            });
        }
        // Add 0 and 1 nodes:
        bdd_nodes.insert(bcdd.terminal_id, (VarID(0), 1, 1));
        bdd_nodes.insert(-bcdd.terminal_id, (VarID(0), 0, 0));

        bdd_nodes
    }
}
