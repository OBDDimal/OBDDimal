/// Representation of a Binary Decision Diagram node, containing the top variable `top_var` and the children `hi` and `lo`).
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Node {
    pub top_var: i64,        // top_variable
    pub low: Box<NodeType>,  // 0 side of the bdd
    pub high: Box<NodeType>, // 1 side of the bdd
}

impl Node {
    pub fn new(v: i64, low: NodeType, high: NodeType) -> NodeType {
        NodeType::Complex(Node {
            top_var: v,
            low: Box::new(low),
            high: Box::new(high),
        })
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum NodeType {
    Zero,
    One,
    Complex(Node),
}
