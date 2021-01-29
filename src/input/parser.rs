use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum FileFormatError {
    InvalidNumber(ParseIntError),
}

/// Takes the path to a DIMACS CNF file and returns a Result containing the `Vec<Vec<i32>>` or a Error why the parse failed.
pub fn parse_file(path: &str) -> Result<Vec<Vec<i32>>, FileFormatError> {
    let mut res = String::new();
    if let Ok(lines) = get_lines_iterator(path) {
        for line in lines {
            if let Ok(ip) = line {
                match ip.chars().nth(0) {
                    Some('c') | Some('p') => continue,
                    Some(_) => res.push_str(&ip),
                    None => continue,
                }
            }
        }
    }
    res.split('0')
        .map(|x| String::from(x.trim()))
        .filter(|x| !x.is_empty())
        .map(|x| x.split(' ').map(|y| y.parse::<i32>().map_err(FileFormatError::InvalidNumber)).collect::<Result<Vec<_>, _>>())
        .collect::<Result<Vec<_>, _>>()
}

fn get_lines_iterator<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}