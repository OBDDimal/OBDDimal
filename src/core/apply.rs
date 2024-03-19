//! The Apply operator
use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, VarID, ONE, ZERO},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ApplyOperation {
    AND,
    OR,
    XOR,
}
struct ApplyOperationFunctions {
    both_terminal: fn(&mut DDManager, NodeID, NodeID) -> NodeID,
    first_terminal: fn(&mut DDManager, NodeID, NodeID) -> NodeID,
    second_terminal: fn(&mut DDManager, NodeID, NodeID) -> NodeID,
    both_equal: fn(&mut DDManager, NodeID) -> NodeID,
}
const fn get_apply_operation_functions(op: ApplyOperation) -> ApplyOperationFunctions {
    const Z_ID: NodeID = ZERO.id;
    const O_ID: NodeID = ONE.id;
    match op {
        ApplyOperation::AND => {
            const BOTH_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |_, u, v| match (u, v) {
                    (Z_ID, Z_ID) => Z_ID,
                    (Z_ID, O_ID) => Z_ID,
                    (O_ID, Z_ID) => Z_ID,
                    (O_ID, O_ID) => O_ID,
                    _ => panic!("Apply operation both_terminal applied to a non-terminal node!"),
                };
            const FIRST_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |_, u, v| if u == Z_ID { Z_ID } else { v };
            const SECOND_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |man, u, v| FIRST_TERMINAL(man, v, u);
            const BOTH_EQUAL: fn(&mut DDManager, NodeID) -> NodeID = |_, u| u;
            ApplyOperationFunctions {
                both_terminal: BOTH_TERMINAL,
                first_terminal: FIRST_TERMINAL,
                second_terminal: SECOND_TERMINAL,
                both_equal: BOTH_EQUAL,
            }
        }
        ApplyOperation::OR => {
            const BOTH_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |_, u, v| match (u, v) {
                    (Z_ID, Z_ID) => Z_ID,
                    (Z_ID, O_ID) => O_ID,
                    (O_ID, Z_ID) => O_ID,
                    (O_ID, O_ID) => O_ID,
                    _ => panic!("Apply operation both_terminal applied to a non-terminal node!"),
                };
            const FIRST_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |_, u, v| if u == Z_ID { v } else { O_ID };
            const SECOND_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |man, u, v| FIRST_TERMINAL(man, v, u);
            const BOTH_EQUAL: fn(&mut DDManager, NodeID) -> NodeID = |_, u| u;
            ApplyOperationFunctions {
                both_terminal: BOTH_TERMINAL,
                first_terminal: FIRST_TERMINAL,
                second_terminal: SECOND_TERMINAL,
                both_equal: BOTH_EQUAL,
            }
        }
        ApplyOperation::XOR => {
            const BOTH_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |_, u, v| match (u, v) {
                    (Z_ID, Z_ID) => Z_ID,
                    (Z_ID, O_ID) => O_ID,
                    (O_ID, Z_ID) => O_ID,
                    (O_ID, O_ID) => Z_ID,
                    _ => panic!("Apply operation both_terminal applied to a non-terminal node!"),
                };
            const FIRST_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |man, u, v| if u == Z_ID { v } else { man.not(v) };
            const SECOND_TERMINAL: fn(&mut DDManager, NodeID, NodeID) -> NodeID =
                |man, u, v| FIRST_TERMINAL(man, v, u);
            const BOTH_EQUAL: fn(&mut DDManager, NodeID) -> NodeID = |_, _| Z_ID;
            ApplyOperationFunctions {
                both_terminal: BOTH_TERMINAL,
                first_terminal: FIRST_TERMINAL,
                second_terminal: SECOND_TERMINAL,
                both_equal: BOTH_EQUAL,
            }
        }
    }
}

impl DDManager {
    pub fn apply(&mut self, op: ApplyOperation, u: NodeID, v: NodeID) -> NodeID {
        if let Some(result) = self.apply_c_table.get(&(op, u, v)) {
            return *result;
        }

        let u = *self
            .nodes
            .get(&u)
            .expect("Apply called on non-existing Nodes!");
        let v = *self
            .nodes
            .get(&v)
            .expect("Apply called on non-existing Nodes!");

        let result = if u.var == VarID(0) && v.var == VarID(0) {
            // Both nodes are terminal nodes
            let both_terminal = get_apply_operation_functions(op).both_terminal;
            both_terminal(self, u.id, v.id)
        } else if u.var == VarID(0) {
            // First node is terminal node
            (get_apply_operation_functions(op).first_terminal)(self, u.id, v.id)
        } else if v.var == VarID(0) {
            // Second node is terminal node
            (get_apply_operation_functions(op).second_terminal)(self, u.id, v.id)
        } else if u.id == v.id {
            // Both nodes are equal
            (get_apply_operation_functions(op).both_equal)(self, u.id)
        } else {
            // No special case
            let upper_var = if self.var2level[u.var.0] <= self.var2level[v.var.0] {
                u.var
            } else {
                v.var
            };
            let (u_high, u_low) = if u.var == upper_var {
                (u.high, u.low)
            } else {
                (u.id, u.id)
            };
            let (v_high, v_low) = if v.var == upper_var {
                (v.high, v.low)
            } else {
                (v.id, v.id)
            };

            let w_high = self.apply(op, u_high, v_high);
            let w_low = self.apply(op, u_low, v_low);

            self.node_get_or_create(&DDNode {
                id: NodeID(0),
                var: upper_var,
                high: w_high,
                low: w_low,
            })
        };

        self.apply_c_table.insert((op, u.id, v.id), result);
        result
    }
}
