use std::collections::BTreeSet;
use std::num::ParseIntError;

//TODO: Implement error trait
#[derive(Debug, Eq, PartialEq)]
pub enum DataFormatError {
    InvalidNumber(ParseIntError),
    MultipleHeaders,
    NonAscendingVariables,
    MissingHeader,
    InvalidHeaderFormat,
    InvalidHeaderData(HeaderDataType),
}

//TODO: Implement error trait
#[derive(Debug, Eq, PartialEq)]
pub enum HeaderDataType {
    VariableCount,
    TermCount,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Cnf {
    pub varibale_count: u32,
    pub term_count: u32,
    pub terms: Vec<Vec<i32>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParserSettings {
    pub ignore_header: bool,
    pub ignore_variable_count: bool,
    pub ignore_term_count: bool,
    pub ignore_ascending_variables: bool,
}

impl Default for ParserSettings {
    fn default() -> Self {
        Self {
            ignore_header: false,
            ignore_variable_count: false,
            ignore_term_count: false,
            ignore_ascending_variables: false,
        }
    }
}

/// Takes a `&str` and returns a `Result<Vec<Vec<i32>>, DataFormatError>`.
/// The ok part of the result contains a vector containing all the clauses of the
/// given `input` as vectors.
pub fn parse_string(input: &str, settings: ParserSettings) -> Result<Cnf, DataFormatError> {
    let headers = input
        .split('\n')
        .filter(|l| matches!(l.chars().next(), Some('p')))
        .collect::<Vec<_>>();

    match headers.len() {
        0 => {
            if !settings.ignore_header {
                Err(DataFormatError::MissingHeader)?
            }
        }
        1 => {}
        _ => Err(DataFormatError::MultipleHeaders)?,
    }

    let lines = input
        .split('\n')
        .filter(|l| !matches!(l.chars().next(), Some('p') | Some('c')));

    let mut buff = String::new();

    for l in lines {
        buff.push_str(l);
        buff.push(' ');
    }

    let mut terms = vec![];
    let mut current = vec![];

    for ele in buff.split_whitespace().filter(|x| !x.is_empty()) {
        if ele != "0" {
            current.push(ele.parse::<i32>().map_err(DataFormatError::InvalidNumber)?);
        } else {
            terms.push(current);
            current = vec![];
        }
    }

    let term_count = terms.len();

    let vars = terms
        .iter()
        .flatten()
        .map(|x| x.abs())
        .collect::<BTreeSet<_>>();

    let var_count = vars.len();

    if !settings.ignore_ascending_variables {
        let is_ascending = vars
            .iter()
            .zip(vars.iter().skip(1))
            .all(|(&lhs, &rhs)| (lhs + 1) == rhs);

        if !is_ascending {
            Err(DataFormatError::NonAscendingVariables)?;
        }
    }

    if !settings.ignore_header {
        let header = headers[0]; // Can not panic, because length is exactly 1.
        let mut split_header = header.split_whitespace();
        let _ = split_header.next();
        let _cnf: &str = split_header
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)?;
        let header_var_count = split_header
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)
            .and_then(|x| x.parse::<i32>().map_err(DataFormatError::InvalidNumber))?;
        let header_term_count = split_header
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)
            .and_then(|x| x.parse::<i32>().map_err(DataFormatError::InvalidNumber))?;

        if header_var_count != var_count as i32 {
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::VariableCount,
            ))?;
        }

        if header_term_count != term_count as i32 {
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::TermCount,
            ))?;
        }
    }
    Ok(Cnf {
        varibale_count: var_count as u32,
        term_count: term_count as u32,
        terms: terms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_input_parse() {
        let input = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default()).unwrap();

        assert_eq!(output.terms, vec![vec![1, -3], vec![2, 3, -1]])
    }

    #[test]
    fn non_ascending_parse() {
        let input = std::fs::read_to_string("examples/assets/nonascending.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::NonAscendingVariables));
    }

    #[test]
    fn non_ascending_ignored_parse() {
        let input = std::fs::read_to_string("examples/assets/nonascending.dimacs").unwrap();
        let output = parse_string(
            &input,
            ParserSettings {
                ignore_ascending_variables: true,
                ..ParserSettings::default()
            },
        )
        .unwrap();

        assert_eq!(output.terms, vec![vec![1, 3]]);
    }

    #[test]
    fn multiple_header_parse() {
        let input = "p cnf 1 1\np cnf 2 1\n1 2 0";
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::MultipleHeaders));
    }

    #[test]
    fn ignore_header_parse() {
        let input = std::fs::read_to_string("examples/assets/ignoreheader.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::MissingHeader));
    }

    #[test]
    fn invalid_header_format_parse() {
        let input = std::fs::read_to_string("examples/assets/invalidheaderformat.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::InvalidHeaderFormat));
    }

    #[test]
    fn invalid_header_format_ignored_parse() {
        let input = std::fs::read_to_string("examples/assets/ignoreheader.dimacs").unwrap();
        let output = parse_string(
            &input,
            ParserSettings {
                ignore_header: true,
                ..ParserSettings::default()
            },
        )
        .unwrap();

        assert_eq!(output.terms, vec![vec![1, 2]]);
    }

    #[test]
    fn invalid_header_data_variable_parse() {
        let input = std::fs::read_to_string("examples/assets/invalidheaderdata.dimacs").unwrap();
        let output = parse_string(
            &input,
            ParserSettings {
                ..ParserSettings::default()
            },
        );

        assert_eq!(
            output,
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::VariableCount
            ))
        );
    }

    #[test]
    fn invalid_header_data_term_parse() {
        let input = std::fs::read_to_string("examples/assets/invalidheaderdata2.dimacs").unwrap();
        let output = parse_string(
            &input,
            ParserSettings {
                ..ParserSettings::default()
            },
        );

        assert_eq!(
            output,
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::TermCount
            ))
        );
    }

    #[test]
    fn ignore_header_true_parse() {
        let input = std::fs::read_to_string("examples/assets/ignoreheader.dimacs").unwrap();
        let output = parse_string(
            &input,
            ParserSettings {
                ignore_header: true,
                ..ParserSettings::default()
            },
        )
        .unwrap();

        assert_eq!(output.terms, vec![vec![1, 2]]);
    }

    #[test]
    fn crooked_input_parse() {
        let input = std::fs::read_to_string("examples/assets/crooked.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default()).unwrap();

        assert_eq!(output.terms, vec![vec![1, -3, 2, 3, 2], vec![1]]);
    }
}
