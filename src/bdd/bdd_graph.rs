use std::rc::Rc;

/// Representation of a Binary Decision Diagram node, containing the top variable `top_var` and the children `hi` and `lo`.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Node {
    pub top_var: i64,       // top_variable
    pub low: Rc<NodeType>,  // 0 side of the bdd
    pub high: Rc<NodeType>, // 1 side of the bdd
}

impl Node {
    pub fn new(v: i64, low: Rc<NodeType>, high: Rc<NodeType>) -> NodeType {
        NodeType::Complex(Node {
            top_var: v,
            low: low,
            high: high,
        })
    }
}

/// Representation of what types a `Node` in a BDD can be.
/// `Zero` is the terminal 0 node.
/// `One` is the terminal 1 node.
/// `Complex(Node)` represents a `Node`.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum NodeType {
    Zero,
    One,
    Complex(Node),
}
