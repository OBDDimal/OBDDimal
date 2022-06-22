use super::bdd_node::DDNode;
use super::dimacs::Instance;

use rustc_hash::{FxHashMap, FxHashSet};

use num_bigint::BigUint;
use num_traits::{One, Zero};

use rand::Rng;

pub struct DDManager {
    pub nodes: FxHashMap<u32, DDNode>,
    order: Vec<u32>,
    var2nodes: Vec<FxHashSet<DDNode>>,
    c_table: FxHashMap<(u32, u32, u32), u32>,
}

impl Default for DDManager {
    fn default() -> Self {
        let mut man = DDManager {
            nodes: FxHashMap::default(),
            order: Vec::new(),
            var2nodes: Vec::new(),
            c_table: FxHashMap::default(),
        };

        man.bootstrap();
        man
    }
}

pub fn align_clauses(clauses: &[Vec<i32>], _order: &[u32]) -> Vec<usize> {
    let mut shuffle: Vec<(usize, f32)> = Vec::default();

    for (i, clause) in clauses.iter().enumerate() {
        for x in clause {
            let _y = x.abs();
        }

        let min = clause.iter().map(|x| x.abs()).min().unwrap();
        let max = clause.iter().map(|x| x.abs()).max().unwrap();

        shuffle.push((i, (clause.len() as f32 * (max - min) as f32)));
    }

    shuffle.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());
    shuffle.iter().map(|(x, _)| *x).collect::<Vec<usize>>()
}

impl DDManager {
    pub fn from_instance(instance: &mut Instance, order: Option<Vec<u32>>) -> (DDManager, u32) {
        let mut man = DDManager::default();
        if let Some(o) = order {
            man.order = o;
            instance.clause_order = Some(align_clauses(&instance.clauses, &man.order));
        }

        let mut bdd = man.one();

        let mut index_giver = (0..instance.clauses.len()).collect::<Vec<usize>>();

        if instance.clause_order.is_some() {
            index_giver = align_clauses(&instance.clauses, &man.order)
        }

        let iter = index_giver.iter_mut();

        let mut n = 1;
        for i in iter {
            let clause = &instance.clauses[*i];

            println!("{:?}", clause);

            let mut cbdd = man.zero();
            for x in clause {
                let node = if *x < 0_i32 {
                    man.nith_var(-x as u32)
                } else {
                    man.ith_var(*x as u32)
                };

                cbdd = man.or(node, cbdd);
            }

            bdd = man.and(cbdd, bdd);

            man.purge_retain(bdd);

            println!(
                "{:?} ({:?}/{:?})",
                &man.nodes.len(),
                n,
                &instance.clauses.len()
            );
            n += 1;
        }

        (man, bdd)
    }

    fn bootstrap(&mut self) {
        let zero = Box::new(DDNode {
            id: 0,
            var: 0,
            low: 0,
            high: 0,
            misc: 0,
        });
        let one = Box::new(DDNode {
            id: 1,
            var: 0,
            low: 1,
            high: 1,
            misc: 0,
        });

        self.add_node(zero);
        self.add_node(one);
    }

    fn ensure_order(&mut self, target: usize) {
        let old_size = self.order.len();

        if target < old_size {
            return;
        }

        self.order.resize(target + 1, 0);
        let mut y = old_size;

        for x in old_size..self.order.len() {
            self.order[x] = y as u32;
            y += 1;
        }

        self.order[0] = y as u32;

        println!("RESIZE: {:?}", self.order);
    }

    fn add_node(&mut self, mut node: Box<DDNode>) -> u32 {
        if node.id == 0 && node.var != 0 {
            let mut id = rand::thread_rng().gen::<u32>();

            while self.nodes.get(&id).is_some() {
                id = rand::thread_rng().gen::<u32>();
            }

            node.id = id;
        }

        let id = node.id;
        let var = node.var;

        let node2 = node.clone();

        self.nodes.insert(id, *node);

        while self.var2nodes.len() <= (var as usize) {
            self.var2nodes.push(FxHashSet::default())
        }

        self.ensure_order(var as usize);

        self.var2nodes[var as usize].insert(*node2);

        id
    }

    fn node_get_or_create(&mut self, node: Box<DDNode>) -> u32 {
        if self.var2nodes.len() <= (node.var as usize) {
            return self.add_node(node);
        }

        let res = self.var2nodes[node.var as usize].get(&*node);

        match res {
            Some(stuff) => stuff.id,
            None => self.add_node(node),
        }
    }

    #[allow(dead_code)]
    fn is_sat(&self, node: u32) -> bool {
        node != 0
    }

    //------------------------------------------------------------------------//
    // Constants

    fn zero(&self) -> u32 {
        0
    }

    fn one(&self) -> u32 {
        1
    }

    //------------------------------------------------------------------------//
    // Variables

    pub fn ith_var(&mut self, var: u32) -> u32 {
        let v = Box::new(DDNode {
            id: 0,
            var,
            low: 0,
            high: 1,
            misc: 0,
        });

        if self.var2nodes.len() > (var as usize) {
            let x = self.var2nodes[var as usize].get(&*v);

            if let Some(x) = x {
                return x.id;
            }
        }

        self.add_node(v)
    }

    pub fn nith_var(&mut self, var: u32) -> u32 {
        let v = Box::new(DDNode {
            id: 0,
            var,
            low: 1,
            high: 0,
            misc: 0,
        });

        if self.var2nodes.len() > (var as usize) {
            let x = self.var2nodes[var as usize].get(&*v);

            if let Some(x) = x {
                return x.id;
            }
        }

        self.add_node(v)
    }

    //------------------------------------------------------------------------//
    // Unitary Operations

    fn not(&mut self, f: u32) -> u32 {
        self.ite(f, 0, 1)
    }

    //------------------------------------------------------------------------//
    // Binary Operations

    pub fn and(&mut self, f: u32, g: u32) -> u32 {
        self.ite(f, g, 0)
    }

    pub fn or(&mut self, f: u32, g: u32) -> u32 {
        self.ite(f, 1, g)
    }

    #[allow(dead_code)]
    fn xor(&mut self, f: u32, g: u32) -> u32 {
        let ng = self.not(g);

        self.ite(f, ng, g)
    }

    //------------------------------------------------------------------------//
    // N-ary Operations

    fn min_by_order(&self, fvar: u32, gvar: u32, hvar: u32) -> u32 {
        let list: Vec<u32> = vec![fvar, gvar, hvar]
            .into_iter()
            .filter(|x| *x > 0)
            .collect::<Vec<u32>>();

        // println!("{:?}", list);
        let tlist = list
            .iter()
            .map(|&x| self.order[x as usize])
            .collect::<Vec<u32>>();
        // println!("{:?}", list);

        let min: u32 = *tlist.iter().min().unwrap();
        let index = tlist.iter().position(|&x| x == min).unwrap();

        list[index]
    }

    fn ite(&mut self, f: u32, g: u32, h: u32) -> u32 {
        match (f, g, h) {
            (_, 1, 0) => f,
            (_, 0, 1) => self.not(f),
            (1, _, _) => g,
            (0, _, _) => h,
            (_, t, e) if t == e => g,
            (_, _, _) => {
                let cache = self.c_table.get(&(f, g, h));

                if let Some(cached) = cache {
                    return *cached;
                }

                let fnode = &self.nodes.get(&f).unwrap();
                let gnode = &self.nodes.get(&g).unwrap();
                let hnode = &self.nodes.get(&h).unwrap();

                let top: u32 = self.min_by_order(fnode.var, gnode.var, hnode.var);

                let fxt = fnode.restrict(top, &self.order, true);
                let gxt = gnode.restrict(top, &self.order, true);
                let hxt = hnode.restrict(top, &self.order, true);

                let fxf = fnode.restrict(top, &self.order, false);
                let gxf = gnode.restrict(top, &self.order, false);
                let hxf = hnode.restrict(top, &self.order, false);

                let high = self.ite(fxt, gxt, hxt);
                let low = self.ite(fxf, gxf, hxf);

                if low == high {
                    return low;
                }

                let node = Box::new(DDNode {
                    id: 0,
                    var: top,
                    low,
                    high,
                    misc: 0,
                });

                let out = self.node_get_or_create(node);

                self.c_table.insert((f, g, h), out);

                out
            }
        }
    }

    //------------------------------------------------------------------------//
    // Builders

    /// Creates an XOR "ladder"
    ///
    ///
    #[allow(dead_code)]
    fn xor_prim(&mut self, _vars: Vec<u32>) -> u32 {
        todo!();
    }

    #[allow(dead_code)]
    fn verify(&self, f: u32, trues: Vec<u32>) -> bool {
        let mut values: Vec<bool> = vec![false; self.var2nodes.len() + 1];

        for x in trues {
            let x: usize = x as usize;

            if x < values.len() {
                values[x] = true;
            } else {
                values[x] = false;
            }
        }

        let mut node_id = f;

        while node_id >= 2 {
            let node = &self.nodes.get(&node_id).unwrap();

            if values[node.var as usize] {
                node_id = node.high;
            } else {
                node_id = node.low;
            }
        }

        node_id == 1
    }

    pub fn sat_count(&self, f: u32) -> BigUint {
        self.sat_count_rec(f, &mut FxHashMap::default())
    }

    fn sat_count_rec(&self, f: u32, cache: &mut FxHashMap<u32, BigUint>) -> BigUint {
        let mut total: BigUint = Zero::zero();
        let node_id = f;

        if node_id == 0 {
            return Zero::zero();
        } else if node_id == 1 {
            return One::one();
        } else {
            let node = &self.nodes.get(&node_id).unwrap();

            let low = &self.nodes.get(&node.low).unwrap();
            let high = &self.nodes.get(&node.high).unwrap();

            let low_jump = if low.var == 0 {
                self.order.len() as u32 - self.order[node.var as usize] - 1
            } else {
                self.order[low.var as usize] - self.order[node.var as usize] - 1
            };

            let high_jump = if high.var == 0 {
                self.order.len() as u32 - self.order[node.var as usize] - 1
            } else {
                self.order[high.var as usize] - self.order[node.var as usize] - 1
            };

            let low_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(low_jump);
            let high_fac = BigUint::parse_bytes(b"2", 10).unwrap().pow(high_jump);

            total += match cache.get(&node.low) {
                Some(x) => x * low_fac,
                None => self.sat_count_rec(node.low, cache) * low_fac,
            };

            total += match cache.get(&node.high) {
                Some(x) => x * high_fac,
                None => self.sat_count_rec(node.high, cache) * high_fac,
            };
        };

        cache.insert(f, total.clone());

        total
    }

    #[allow(dead_code)]
    pub fn count_active(&self, f: u32) -> u32 {
        let mut nodes: FxHashSet<u32> = FxHashSet::default();

        let mut stack: Vec<u32> = vec![f];

        while !stack.is_empty() {
            let x = stack.pop().unwrap();

            if nodes.contains(&x) {
                continue;
            }

            let node = self.nodes.get(&x).unwrap();

            stack.push(node.low);
            stack.push(node.high);
            nodes.insert(x);
        }

        nodes.len() as u32
    }

    pub fn purge_retain(&mut self, f: u32) {
        let mut keep: FxHashSet<u32> = FxHashSet::default();

        let mut stack: Vec<u32> = vec![f];

        while !stack.is_empty() {
            let x = stack.pop().unwrap();

            if keep.contains(&x) {
                continue;
            }

            let node = self.nodes.get(&x).unwrap();

            stack.push(node.low);
            stack.push(node.high);
            keep.insert(x);
        }

        let mut garbage = self.nodes.clone();

        garbage.retain(|&x, _| !keep.contains(&x) && x > 1);

        for x in &garbage {
            self.var2nodes[x.1.var as usize].remove(x.1);
            self.nodes.remove(x.0);
        }

        self.c_table.retain(|_, x| keep.contains(x));
    }
}
