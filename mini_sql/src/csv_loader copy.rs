use std::error::Error;

use crate::table::{Column, DataType, Table};

pub fn load_csv(path_csv: &str) -> Result<Table, Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(path_csv)?;

    let columns: Vec<Column> = reader
        .headers()?
        .iter()
        .map(|name| Column {
            name: name.to_string(),
            data_type: DataType::String,
        })
        .collect();

    let mut rows = Vec::new();

    for record in reader.records() {
        let record = record?;
        rows.push(record.iter().map(String::from).collect());
    }

    Ok(Table { columns, rows })
}