use std::fs;
use std::error::Error;

use crate::table::Table;

pub fn load_csv(path_csv: &str) -> Result<Table, Box<dyn Error>> {
    let message: String = fs::read_to_string(path_csv)?;

    for line in message.lines() {
        println!("{}", line);
    }
    Ok(Table::new())

}

