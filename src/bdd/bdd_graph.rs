/// Representation of a Binary Decision Diagram node, containing the top variable `top_var` and the children `hi` and `lo`).
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Node {
    pub   top_var: u32,          // top_variable
    pub      high: Option<Box<Node>>,    // 1 side of the bdd
    pub       low: Option<Box<Node>>,    // 0 side of the bdd
}


