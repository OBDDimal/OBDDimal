use std::num::ParseIntError;

//TODO: Implement error trait
#[derive(Debug)]
pub enum DataFormatError {
    InvalidNumber(ParseIntError),
}

/// Takes a `&str` and returns a `Result<Vec<Vec<i32>>, DataFormatError>`.
/// The ok part of the result contains a vector containing all the clauses of the
/// given `input` as vectors.
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

#[cfg(test)]
mod tests {
    #[test]
    fn small_input_parse() {
        let input = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
        let output = super::parse_string(&input).unwrap();

        assert_eq!(output, vec![vec![1, -3], vec![2, 3, -1]])
    }

    #[test]
    fn croocked_input_parse() {
        let input = std::fs::read_to_string("examples/assets/croocked.dimacs").unwrap();
        let output = super::parse_string(&input).unwrap();

        assert_eq!(output, vec![vec![1, -3, 2, 3, 4], vec![100]])
    }
}
