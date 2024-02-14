//! Utility functions related to variable order

use dimacs::Instance;

use crate::core::bdd_node::VarID;

/// Checks if a specified variable ordering is valid for the CNF instance.
/// Returns `OK(())` or `Err("error message")`.
pub(crate) fn check_order(cnf: &Instance, order: &[usize]) -> Result<(), String> {
    let num_vars = match cnf {
        Instance::Cnf { num_vars, .. } => *num_vars as usize,
        _ => panic!("Unsupported dimacs format!"),
    };
    if order.len() != num_vars + 1 {
        return Err(format!(
            "Invalid size of ordering: Size was {}, expected {} (nr of variables + 1)",
            order.len(),
            num_vars + 1
        ));
    }

    if order[0] != num_vars + 1 {
        return Err(format!(
            "Depth of terminal nodes (index 0) is specified as {}, but should be {} (nr of variables + 1)",order[0], num_vars+1
        ));
    }

    let max_depth = *order.iter().max().unwrap();
    if order[0] != max_depth {
        return Err(format!(
            "A variable is specified to have depth of {} which is below depth \
            of terminal nodes ({}, index 0)",
            max_depth, order[0]
        ));
    }

    let mut var_map = vec![0; num_vars + 1];
    for (var, depth) in order.iter().enumerate() {
        if *depth < 1 {
            return Err(format!(
                "Variable {} specified at depth {} which is < 1",
                var, depth
            ));
        }

        if *depth > num_vars && var != 0 {
            return Err(format!(
                "Variable {} specified at depth {} which is greater than the number of variables",
                var, depth
            ));
        }

        var_map[*depth - 1] = var;
    }

    for (depth, var) in var_map.iter().enumerate() {
        if *var == 0 && depth != num_vars {
            return Err(format!("No variable at depth {}", depth + 1));
        }
    }

    Ok(())
}

/// Returns the variable order as list of VarID top to bottom
pub(crate) fn order_to_layernames(order: &[usize]) -> Vec<VarID> {
    let mut res = vec![VarID(0); *order.iter().max().unwrap()];
    for (var_num, var_pos) in order.iter().enumerate() {
        res[*var_pos - 1] = VarID(var_num);
    }
    res
}

#[cfg(test)]
mod tests {
    use crate::core::bdd_node::VarID;

    #[test]
    fn order_to_layernames() {
        let res = super::order_to_layernames(&[4, 1, 3, 2]);
        assert_eq!(res, vec![VarID(1), VarID(3), VarID(2), VarID(0)])
    }
}
