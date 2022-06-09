mod core;

use crate::core::bdd_manager::DDManager;

use std::{
    fs::File,
    io::{prelude::*, BufReader},
    path::Path,
};

use regex::Regex;

use rand::thread_rng;
use rand::seq::SliceRandom;

#[derive(Debug)]
pub struct Instance {
    pub no_clauses:u32,
    pub no_variables:u32,
    pub clauses:Vec<Vec<i32>>,
    pub clause_order:Option<Vec<usize>>
}


impl Instance {

    fn new(no_clauses:u32, no_variables:u32, clauses:Vec<Vec<i32>>) -> Self {
        Instance {
            no_clauses: no_clauses,
            no_variables: no_variables,
            clauses: clauses,
            clause_order: None
        }
    }
}

fn file_readlines(filename: impl AsRef<Path>) -> Vec<String> {

    let file = File::open(filename).expect("File not found.");
    let buffer = BufReader::new(file);
    buffer.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

fn parse_dimacs(filename: impl AsRef<Path>) -> Instance {

    let lines = file_readlines(&filename);

    let mut header_parsed = false;
    let mut no_variables:u32 = 0;
    let mut no_clauses:u32 = 0;

    let mut variables:Vec<(u32, String)> = Vec::new();
    let mut clauses:Vec<Vec<i32>> = Vec::new();

    let re_c = Regex::new(r"^c\s+(?P<var_id>\d+)\s+(?P<var_name>[\w\s_+/]+)\s*$").unwrap();
    let re_p = Regex::new(r"^p\s+(?P<type>[\w]+)\s+(?P<no_variables>\d+)\s+(?P<no_clauses>\d+)$").unwrap();
    let re_clause = Regex::new(r"^([-]?\d+\s+)+0$").unwrap();

    let re_clause_split = Regex::new(r"\s+").unwrap();

    for line in lines {
        let mut m = re_c.captures(&line);

        if m.is_some() {

            let cap = m.unwrap();
            let var_id = &cap["var_id"].parse::<u32>().unwrap();  
            let var_name = String::from(&cap["var_name"]).clone();

            variables.push((*var_id, var_name));
            continue;
        }

        if !header_parsed {
            m = re_p.captures(&line);
            if m.is_some() {

                let cap = m.unwrap();
                
                no_variables = cap["no_variables"].parse::<u32>().unwrap(); 
                no_clauses = cap["no_clauses"].parse::<u32>().unwrap(); 
                header_parsed = true;
                continue;
            } 
        }       

        m = re_clause.captures(&line);

        if m.is_some() {
            let mut vars_raw: Vec<&str> = re_clause_split.split(&line).collect();

            if vars_raw.pop() != Some("0") {
                panic!("Last element of clause was not 0");
            }

            let mut vars: Vec<i32> = vars_raw.iter().map(|x| x.parse::<i32>().unwrap()).collect::<Vec<i32>>();
            vars.sort();

            clauses.push(vars);

            // println!("{:?}", vars)

        }else{
            panic!("Unknown line type {:?}", &line);
        }

    }

    // println!("{:?}", no_variables);
    // println!("{:?}", no_clauses);
    // println!("{:?}", clauses);

    Instance::new(no_clauses, no_variables, clauses)
}

fn keep(instance: &Instance) -> Vec<u32> {
    let mut order:Vec<u32> = (1..instance.no_variables+1).collect();

    order.insert(0, (order.len()+1) as u32);
    order
}

fn rand(instance: &Instance) -> Vec<u32> {
    let mut order:Vec<u32> = (1..instance.no_variables+1).collect();

    order.shuffle(&mut thread_rng());

    order.insert(0, (order.len()+1) as u32);
    order
}

fn force(instance: &Instance) -> Vec<u32> {
    let mut order:Vec<u32> = (1..instance.no_variables+1).collect();

    order.shuffle(&mut thread_rng());
    order.insert(0, (order.len()+1) as u32);

    let mut span:Option<u32> = None;

    let mut converged = false;

    let mut n = 0;

    while !converged && n < 1000{

        n+=1;

        let mut tpos:Vec<f64> = vec![0.0;(instance.no_variables+1) as usize];
        let mut degree:Vec<u32> = vec![0;(instance.no_variables+1) as usize];

        for clause in &instance.clauses {
            let cog:f64 = calc_center_of_gravity(&clause, &order);

            for x in clause {
                let y = x.abs() as usize;
                tpos[y] += cog;
                degree[y] += 1;
            }
        }

        for x in 1..instance.no_variables+1 {
            let y = x as usize;
            tpos[y] = tpos[y] / (degree[y] as f64);
        }

        // println!("{:?}", tpos);

        order = (1..instance.no_variables+1).collect::<Vec<u32>>();

        // println!("{:?}", order);

        order.sort_by(|x,y| tpos[*x as usize].partial_cmp(&tpos[*y as usize]).unwrap());
        // println!("{:?}", order);
        order.insert(0, (order.len()+1) as u32);

        let span_new = calc_span(&instance.clauses, &order);

        if span.is_none() || span_new != span.unwrap() {
            span = Some(span_new);
        }else{
            converged = true;
        }

        println!("{:?}", span);
    }

    order
}

fn calc_center_of_gravity(clause: &Vec<i32>, order:&Vec<u32>) -> f64 {
    let mut out = 0;
    for x in clause {
        out += order[x.abs() as usize];
    }

    out as f64 / clause.len() as f64
}

fn calc_span(clauses: &Vec<Vec<i32>>, order:&Vec<u32>) -> u32 {

    let mut span = 0;

    for clause in clauses {
        let pos = clause.iter().map(|x| order[x.abs() as usize] as u32).collect::<Vec<u32>>();
        let min = pos.iter().min().unwrap();
        let max = pos.iter().max().unwrap();
        span += max - min;
    }

    span
}

fn order_dist(instance:&Instance) -> Vec<u32> {
    let n = (instance.no_variables+1)as usize;
    let mut dists:Vec<Vec<u32>> = vec![vec![0;n];n];
    for clause in &instance.clauses {
        for x_ in clause {
            let x = x_.abs() as usize;

            for y_ in clause {
                let y = y_.abs() as usize;
                if x >= y {
                    break;
                }

                dists[x][y] += 1;
                dists[y][x] += 1;
            }
        }
    }

    println!("{:?}", dists.iter().enumerate().map(|(i,x)| (i, x.iter().sum::<u32>())).max_by(|(_,x), (_,y)| x.partial_cmp(y).unwrap()).unwrap());


    vec![0]
}

fn main() {

    let mut man = DDManager::default();

    // let mut instance = parse_dimacs("examples/cerf.dimacs");
    // let mut instance = parse_dimacs("examples/sandwich.dimacs");
    let mut instance = parse_dimacs("examples/berkeleydb.dimacs");
    // let instance = parse_dimacs("examples/busybox.dimacs");

    let order = rand(&instance);
    // println!("{:?}", order);

    // println!("{:?}", instance);

    let bdd = man.from_instance(&mut instance, Some(order));

    // println!("{:?}", man.nodes.len());

    // man.purge_retain(bdd);

    // println!("{:?}", man.nodes.len());
    
    println!("Starting #SAT" );
    println!("{:?}", man.sat_count(bdd));    
}


#[cfg(test)]
mod tests {

    use super::*;
    use num_bigint::{BigUint};


    fn build_verify_ssat(filepath:&str, target:&[u8]){

        let expected = BigUint::parse_bytes(target, 10).unwrap();

        let mut man = DDManager::default();
        let mut instance = parse_dimacs(filepath);
        let bdd = man.from_instance(&mut instance, None);

        assert_eq!(man.sat_count(bdd), expected);
    }

    #[test] 
    fn sandwich_ssat(){
        build_verify_ssat("examples/sandwich.dimacs", b"2808")
    }

    #[test] 
    fn berkeleydb_ssat(){
        build_verify_ssat("examples/berkeleydb.dimacs", b"4080389785")
    }

    #[test]#[ignore] 
    fn busybox_ssat(){
        build_verify_ssat("examples/busybox.dimacs", b"FAIL")
    }
}