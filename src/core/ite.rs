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
        let (f, g, h) = normalize_ite_args(f, g, h);
        match (f, g, h) {
            (_, NodeID(1), NodeID(0)) => f, // ite(f,1,0)
            (NodeID(1), _, _) => g,         // ite(1,g,h)
            (NodeID(0), _, _) => h,         // ite(0,g,h)
            (_, t, e) if t == e => t,       // ite(f,g,g)
            (_, _, _) => {
                let cache = self.ite_c_table.get(&(f, g, h));

                if let Some(cached) = cache {
                    return *cached;
                }

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

                let high = self.ite(fxt, gxt, hxt);
                let low = self.ite(fxf, gxf, hxf);

                if low == high {
                    self.ite_c_table.insert((f, g, h), low);
                    return low;
                }

                let node = DDNode {
                    id: NodeID(0),
                    var: top,
                    low,
                    high,
                };

                let out = self.node_get_or_create(&node);

                self.ite_c_table.insert((f, g, h), out);

                out
            }
        }
    }
}
