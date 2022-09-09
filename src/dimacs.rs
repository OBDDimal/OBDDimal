use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use regex::Regex;

/// Logic formula in conjunctive normal form, parsed from a DIMACS file:
/// A formula in conjunctive normal form is a conjunction (logical and) of a set of clauses.
/// Each clause is a disjunction (logical or) of a set of literals.
/// A literal is a variable or a negation of a variable.
/// (<https://jix.github.io/varisat/manual/0.2.0/formats/dimacs.html#dimacs-cnf>)
#[derive(Debug, Clone)]
pub struct Instance {
    /// Number of clauses as specified in the DIMACS header line
    pub no_clauses: u32,
    /// Number of variables as specified in the DIMACS header line
    pub no_variables: u32,
    pub clauses: Vec<Vec<i32>>,
}

impl Instance {
    fn new(no_clauses: u32, no_variables: u32, clauses: Vec<Vec<i32>>) -> Self {
        Instance {
            no_clauses,
            no_variables,
            clauses,
        }
    }
}

fn file_readlines(filename: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(filename).expect("File not found.");
    let buffer = BufReader::new(file);
    buffer
        .lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

pub fn parse_dimacs(filename: impl AsRef<Path>) -> Instance {
    let lines = file_readlines(&filename);

    let mut header_parsed = false;
    let mut no_variables: u32 = 0;
    let mut no_clauses: u32 = 0;

    let mut variables: Vec<(u32, String)> = Vec::new();
    let mut clauses: Vec<Vec<i32>> = Vec::new();

    let re_c = Regex::new(r"^c\s+(?P<var_id>\d+)\s+(?P<var_name>[\w\s_+/]+)\s*$").unwrap();
    let re_p =
        Regex::new(r"^p\s+(?P<type>[\w]+)\s+(?P<no_variables>\d+)\s+(?P<no_clauses>\d+)$").unwrap();
    let re_clause = Regex::new(r"^([-]?\d+\s+)+0$").unwrap();

    let re_clause_split = Regex::new(r"\s+").unwrap();

    for line in lines {
        if line.starts_with('c') {
            // Lines starting with C are comments.
            // The following additionally tries to parse a comment as a variable name.
            // Note that this currently does not match all variable names found in DIMACS
            // files, which may for example contain special characters or escape sequences.
            let m = re_c.captures(&line);
            if let Some(cap) = m {
                let var_id = &cap["var_id"].parse::<u32>().unwrap();
                let var_name = String::from(&cap["var_name"]).clone();

                variables.push((*var_id, var_name));
            }
            continue;
        }

        if !header_parsed {
            let m = re_p.captures(&line);
            if let Some(cap) = m {
                no_variables = cap["no_variables"].parse::<u32>().unwrap();
                no_clauses = cap["no_clauses"].parse::<u32>().unwrap();
                header_parsed = true;
                continue;
            }
        }

        let m = re_clause.captures(&line);

        if m.is_some() {
            let mut vars_raw: Vec<&str> = re_clause_split.split(&line).collect();

            if vars_raw.pop() != Some("0") {
                panic!("Last element of clause was not 0");
            }

            let mut vars: Vec<i32> = vars_raw
                .iter()
                .map(|x| x.parse::<i32>().unwrap())
                .collect::<Vec<i32>>();
            vars.sort_unstable();

            clauses.push(vars);

            // println!("{:?}", vars)
        } else {
            panic!("Unknown line type {:?}", &line);
        }
    }

    // println!("{:?}", no_variables);
    // println!("{:?}", no_clauses);
    // println!("{:?}", clauses);

    Instance::new(no_clauses, no_variables, clauses)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_comments() {
        let _ = super::parse_dimacs("examples/test_comments.dimacs");
    }
}
