use super::super::error::CliError;

pub(super) fn value_as_string(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<String, CliError> {
    parser
        .value()?
        .into_string()
        .map_err(|_| CliError::message(error_message))
}

pub(super) fn parse_column_list(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<Vec<usize>, CliError> {
    let columns = value_as_string(parser, "Column specification must be valid UTF-8")?;
    columns
        .split(',')
        .map(|part| {
            let column = part.trim().parse::<usize>().map_err(|_| "invalid column")?;
            if column == 0 {
                return Err("column indices are 1-based");
            }
            Ok(column)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CliError::message(error_message))
}
