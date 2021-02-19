use crate::variable_ordering::static_ordering::force;
use std::collections::BTreeSet;
use std::num::ParseIntError;

//TODO: Implement error trait (Somehow it is good practice to not implement the Error trait for those kind of 'high-level' errors.)
#[derive(Debug, Eq, PartialEq)]
pub enum DataFormatError {
    InvalidNumber(ParseIntError),
    MultipleHeaders,
    NonAscendingVariables,
    MissingHeader,
    InvalidHeaderFormat,
    InvalidHeaderData(HeaderDataType),
}

impl std::fmt::Display for DataFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DataFormatError::InvalidHeaderData(header_data_type) => {
                write!(f, "Data in given header is invalid: {}", header_data_type)
            }
            DataFormatError::InvalidNumber(parse_error) => {
                write!(f, "Non-number out of comment line: {}", parse_error)
            }
            DataFormatError::MultipleHeaders => write!(f, "File contains more than one headers."),
            DataFormatError::NonAscendingVariables => {
                write!(f, "Variables are not in ascending order.")
            }
            DataFormatError::MissingHeader => write!(f, "File is missing a header."),
            DataFormatError::InvalidHeaderFormat => write!(f, "File contains a malformed header."),
        }
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum HeaderDataType {
    VariableCount,
    TermCount,
}

impl std::fmt::Display for HeaderDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HeaderDataType::VariableCount => write!(f, "Wrong number of variables."),
            HeaderDataType::TermCount => write!(f, "Wrong number of terms."),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Cnf {
    pub varibale_count: u32,
    pub term_count: u32,
    pub terms: Vec<Vec<i32>>,
    pub order: Vec<i32>,
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

/// Takes a `&str` and returns a `Result<Cnf, DataFormatError>`.
/// The ok part of the result contains a vector containing all the clauses of the
/// given `input` as vectors.
pub fn parse_string(input: &str, settings: ParserSettings) -> Result<Cnf, DataFormatError> {
    // Extract every line starting with the letter p.
    let headers = input
        .split('\n')
        .filter(|l| matches!(l.chars().next(), Some('p')))
        .collect::<Vec<_>>();

    // Check the number of found headers.
    match headers.len() {
        0 => {
            if !settings.ignore_header {
                Err(DataFormatError::MissingHeader)? // If there is no header an error is returned if the setting does not ignore the header.
            }
        }
        1 => {} // If there is exactly 1 header everything is fine.
        _ => Err(DataFormatError::MultipleHeaders)?, // If there are multiple headers an error is returned.
    }

    // Get all lines that are not header or comment lines.
    let lines = input
        .split('\n')
        .filter(|l| !matches!(l.chars().next(), Some('p') | Some('c')));

    // Concatenate all previous found lines with a space.
    let mut buff = String::new();

    for l in lines {
        buff.push_str(l);
        buff.push(' ');
    }

    // Create a vector of vectors containing the variables of the cnf.
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

    // Number of terms in the cnf.
    let term_count = terms.len();

    // Create a ordered set of all the absolute values in the cnf.
    let vars = terms
        .iter()
        .flatten()
        .map(|x| x.abs())
        .collect::<BTreeSet<_>>();

    // Number of all used variables in the cnf.
    let var_count = vars.len();

    if !settings.ignore_ascending_variables {
        // Check if all subsequent variables are ascending by exactly +1.
        let is_ascending = vars
            .iter()
            .zip(vars.iter().skip(1))
            .all(|(&lhs, &rhs)| (lhs + 1) == rhs);

        // If the variables are not ascending and the settings do not ignore them, an error is returned.
        if !is_ascending {
            Err(DataFormatError::NonAscendingVariables)?;
        }
    }

    if !settings.ignore_header {
        let header = headers[0]; // Can not panic, because length is exactly 1.
        let mut split_header = header.split_whitespace(); // Iterator over all elements in the header.
        let _ = split_header.next(); // Discard the first element of the header line 'p'.
        let _cnf: &str = split_header // "cnf" string of the header.
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)?;
        let header_var_count = split_header // Variable count from the header.
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)
            .and_then(|x| x.parse::<i32>().map_err(DataFormatError::InvalidNumber))?;
        let header_term_count = split_header // Term count from the header.
            .next()
            .ok_or(DataFormatError::InvalidHeaderFormat)
            .and_then(|x| x.parse::<i32>().map_err(DataFormatError::InvalidNumber))?;

        // Return an error if the variable count from the cnf header and the calculated variable count does not match.
        if header_var_count != var_count as i32 {
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::VariableCount,
            ))?;
        }

        // Return an error if the term count from the cnf header and the calculated term count does not match.
        if header_term_count != term_count as i32 {
            Err(DataFormatError::InvalidHeaderData(
                HeaderDataType::TermCount,
            ))?;
        }
    }

    let mut cnf = Cnf {
        varibale_count: var_count as u32,
        term_count: term_count as u32,
        terms: terms,
        order: (1_i32..=var_count as i32).collect::<Vec<i32>>(),
    };

    let (order, _span) = force(cnf.clone());

    cnf.order = order;
    // Return the cnf as a struct.
    Ok(cnf)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test small dimacs file.
    #[test]
    fn small_input_parse() {
        let input = std::fs::read_to_string("examples/assets/easy1.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default()).unwrap();

        assert_eq!(output.terms, vec![vec![1, -3], vec![2, 3, -1]])
    }

    // Test if parser detects non ascending variables.
    #[test]
    fn non_ascending_parse() {
        let input = std::fs::read_to_string("examples/assets/nonascending.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::NonAscendingVariables));
    }
    // Test if parser detects the setting to ignore ascending variables.
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
    // Test if parser detects mutliple headers.
    #[test]
    fn multiple_header_parse() {
        let input = "p cnf 1 1\np cnf 2 1\n1 2 0";
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::MultipleHeaders));
    }
    // Test if parser detects missing headers.
    #[test]
    fn missing_header_parse() {
        let input = std::fs::read_to_string("examples/assets/ignoreheader.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::MissingHeader));
    }

    // Test if parser detects invalid headers.
    #[test]
    fn invalid_header_format_parse() {
        let input = std::fs::read_to_string("examples/assets/invalidheaderformat.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default());

        assert_eq!(output, Err(DataFormatError::InvalidHeaderFormat));
    }

    // Test if parser detects settings to ignore invalid headers.
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
    // Test if parser detects settings to ignore invalid header variable count.
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
    // Test if parser detects settings to ignore invalid header term count.
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
    // Test if parser detects settings to ignore headers.
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
    // Test if parser can read crooked input.
    #[test]
    fn crooked_input_parse() {
        let input = std::fs::read_to_string("examples/assets/crooked.dimacs").unwrap();
        let output = parse_string(&input, ParserSettings::default()).unwrap();

        assert_eq!(output.terms, vec![vec![1, -3, 2, 3, 2], vec![1]]);
    }
}
