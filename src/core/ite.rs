//! The ITE operator
use crate::core::{
    bdd_manager::DDManager,
    bdd_node::{DDNode, NodeID, ONE, ZERO},
};

/// Bring ITE calls of the form
/// ite(f,f,h) = ite(f,1,h) = ite(h,1,f)
/// ite(f,g,f) = ite(f,g,0) = ite(g,f,0)
/// into canonical form
fn normalize_ite_args(mut f: NodeID, mut g: NodeID, mut h: NodeID) -> (NodeID, NodeID, NodeID) {
    if f == g {
        g = ONE.id;
    } else if f == h {
        h = ZERO.id
    }

    let order = |a, b| if a < b { (a, b) } else { (b, a) };

    if g == ONE.id {
        (f, h) = order(f, h);
    }
    if h == ZERO.id {
        (f, g) = order(f, g);
    }

    (f, g, h)
}

impl DDManager {
    pub fn ite(&mut self, f: NodeID, g: NodeID, h: NodeID) -> NodeID {
        let mut ite_stack = vec![(f, g, h)];

        while let Some((f, g, h)) = ite_stack.pop() {
            let (f, g, h) = normalize_ite_args(f, g, h);

            if self.ite_c_table.contains_key(&(f, g, h)) {
                continue; // Result already in cache
            }

            let result = match (f, g, h) {
                (_, NodeID(1), NodeID(0)) => Some(f), // ite(f,1,0)
                (NodeID(1), _, _) => Some(g),         // ite(1,g,h)
                (NodeID(0), _, _) => Some(h),         // ite(0,g,h)
                (_, t, e) if t == e => Some(t),       // ite(f,g,g)
                (_, _, _) => {
                    // No special case
                    let fnode = self.nodes.get(&f).unwrap();
                    let gnode = self.nodes.get(&g).unwrap();
                    let hnode = self.nodes.get(&h).unwrap();

                    let top = self.min_by_order(fnode.var, gnode.var, hnode.var);

                    let fxt = fnode.restrict(top, &self.var2level, true);
                    let gxt = gnode.restrict(top, &self.var2level, true);
                    let hxt = hnode.restrict(top, &self.var2level, true);

                    let fxf = fnode.restrict(top, &self.var2level, false);
                    let gxf = gnode.restrict(top, &self.var2level, false);
                    let hxf = hnode.restrict(top, &self.var2level, false);

                    if self.ite_c_table.contains_key(&(fxt, gxt, hxt))
                        && self.ite_c_table.contains_key(&(fxf, gxf, hxf))
                    {
                        let high = *self.ite_c_table.get(&(fxt, gxt, hxt)).unwrap();
                        let low = *self.ite_c_table.get(&(fxf, gxf, hxf)).unwrap();

                        if low == high {
                            Some(low)
                        } else {
                            Some(self.node_get_or_create(&DDNode {
                                id: NodeID(0),
                                var: top,
                                low,
                                high,
                            }))
                        }
                    } else {
                        ite_stack.push((f, g, h));
                        ite_stack.push((fxt, gxt, hxt));
                        ite_stack.push((fxf, gxf, hxf));

                        None
                    }
                }
            };

            if let Some(result) = result {
                self.ite_c_table.insert((f, g, h), result);
            }
        }

        *self.ite_c_table.get(&(f, g, h)).unwrap()
    }
}
