use std::num::ParseIntError;

//TODO: Implement error trait
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
}
