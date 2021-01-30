use std::fs::File;
use std::io::{self, BufRead};
use std::num::ParseIntError;
use std::path::Path;

#[derive(Debug)]
pub enum DataFormatError {
    InvalidNumber(ParseIntError),
}

pub fn parse_string(input: &str) -> Result<Vec<Vec<i32>>, DataFormatError> {
    let lines = input
        .split("\n")
        .filter(|l| !matches!(l.chars().next(), Some('p') | Some('c')));

    let mut buff = String::new();

    for l in lines {
        buff.push_str(l);
        buff.push(' ');
    }

    let mut result = vec![];
    let mut current = vec![];

    for ele in buff.split_whitespace().filter(|x| !x.is_empty()) {
        if ele != "0" {
            current.push(ele.parse::<i32>().map_err(DataFormatError::InvalidNumber)?);
        } else {
            result.push(current);
            current = vec![];
        }
    }

    Ok(result)
    /*
    let mut res = String::new();
    if let Ok(lines) = get_lines_iterator(path) {
        for line in lines {
            if let Ok(ip) = line {
                match ip.chars().next() {
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
        .map(|x| {
            x.split(' ')
                .map(|y| y.parse::<i32>().map_err(FileFormatError::InvalidNumber))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()
        */
}

fn get_lines_iterator<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
