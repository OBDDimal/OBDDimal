//! The Apply operator
#![allow(rustdoc::private_intra_doc_links)]

use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, VarID, ONE, ZERO},
};

/// Enum representing the Operations apply can apply.
///
/// # Adding Operations
/// When an operation is added to this enum, the constant function
/// [`get_apply_operation_functions`] also has to be modified so that it returns the necessary
/// functions for terminal cases in an [`ApplyOperationFunctions`] struct. **If you forget changing
/// the function, the compiler should warn you**.
/// Basically four functions need to be implemented (please take a look at how the existing
/// functions are implemented for a hint on how this is supposed to be done). These four functions
/// are:
/// * `both_terminal`: This function handles the case that both BDDs provided to apply are single
///   terminal nodes.
/// * `first_terminal`: This function handles the case, that only the first BDD is a single
///   terminal node (which can often be handled in constant time).
/// * `second_terminal`: This function handles the case, that only the second BDD is a single
///   terminal node (for symmetric operations, this can just call `first_terminal` with swapped
///   parameters).
/// * `both_equal`: This function serves the case, that both BDDs are equal (which can also usually
///   be handled in constant time).
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
        let mut apply_stack = vec![(op, u, v)];

        while let Some((op, u, v)) = apply_stack.pop() {
            if self.apply_c_table.contains_key(&(op, u, v)) {
                continue; // Result already in cache, so nothing to do
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
                Some(both_terminal(self, u.id, v.id))
            } else if u.var == VarID(0) {
                // First node is terminal node
                Some((get_apply_operation_functions(op).first_terminal)(
                    self, u.id, v.id,
                ))
            } else if v.var == VarID(0) {
                // Second node is terminal node
                Some((get_apply_operation_functions(op).second_terminal)(
                    self, u.id, v.id,
                ))
            } else if u.id == v.id {
                // Both nodes are equal
                Some((get_apply_operation_functions(op).both_equal)(self, u.id))
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

                if self.apply_c_table.contains_key(&(op, u_high, v_high))
                    && self.apply_c_table.contains_key(&(op, u_low, v_low))
                {
                    let w_high = self.apply_c_table.get(&(op, u_high, v_high)).unwrap();
                    let w_low = self.apply_c_table.get(&(op, u_low, v_low)).unwrap();

                    Some(self.node_get_or_create(&DDNode {
                        id: NodeID(0),
                        var: upper_var,
                        high: *w_high,
                        low: *w_low,
                    }))
                } else {
                    apply_stack.push((op, u.id, v.id));
                    apply_stack.push((op, u_high, v_high));
                    apply_stack.push((op, u_low, v_low));
                    None
                }
            };

            if let Some(result) = result {
                self.apply_c_table.insert((op, u.id, v.id), result);
            }
        }

        // The c_table has to contain the result, if the stack is empty
        *self.apply_c_table.get(&(op, u, v)).unwrap()
    }
}
