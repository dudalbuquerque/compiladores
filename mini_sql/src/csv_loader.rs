use std::error::Error;
use std::fs;
use std::io::{Error as IoError, ErrorKind};

use crate::table::{Column, DataType, Table};

pub fn load_csv(path_csv: &str) -> Result<Table, Box<dyn Error>> {
    let content = fs::read_to_string(path_csv)?;
    let mut lines = content.lines();

    let header = lines.next().ok_or_else(|| {
        IoError::new(ErrorKind::InvalidData, "Arquivo CSV vazio")
    })?;

    let column_names: Vec<String> = header
        .split(',')
        .map(|name| name.trim().trim_matches('"').to_string())
        .collect();

    let mut rows: Vec<Vec<String>> = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let row: Vec<String> = line
            .split(',')
            .map(|value| value.trim().trim_matches('"').to_string())
            .collect();

        if row.len() != column_names.len() {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "Quantidade de valores diferente da quantidade de colunas",
            )
            .into());
        }

        rows.push(row);
    }

    let columns = column_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let values: Vec<&str> = rows
                .iter()
                .map(|row| row[index].as_str())
                .filter(|value| !value.is_empty())
                .collect();

            let data_type = if !values.is_empty()
                && values.iter().all(|v| v.parse::<i64>().is_ok())
            {
                DataType::Int
            } else if !values.is_empty()
                && values.iter().all(|v| v.parse::<f64>().is_ok())
            {
                DataType::Float
            } else if !values.is_empty()
                && values.iter().all(|v| {
                    v.eq_ignore_ascii_case("true")
                        || v.eq_ignore_ascii_case("false")
                })
            {
                DataType::Boolean
            } else {
                DataType::String
            };

            Column {
                name: name.clone(),
                data_type,
            }
        })
        .collect();

    Ok(Table { columns, rows })
}
