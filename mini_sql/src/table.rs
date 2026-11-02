use std::collections::HashMap;
use std::fmt;

pub type Database = HashMap<String, Table>;

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    String,
    Int,
    Float,
    Boolean,
}

//A vantagem de isso morar no tipo é que o type checker só pergunta “esses tipos são compatíveis?”, sem repetir a regra

impl DataType {
    /// Int e Float podem ser comparados entre si (o Int é promovido a Float).
    pub fn is_numeric(&self) -> bool {
        matches!(self, DataType::Int | DataType::Float)
    }

    /// Tipos que aceitam <, <=, > e >=.
    pub fn is_ordered(&self) -> bool {
        matches!(self, DataType::Int | DataType::Float | DataType::String)
    }

    /// Dois tipos são compatíveis se forem iguais ou ambos numéricos.
    pub fn is_compatible_with(&self, other: &DataType) -> bool {
        self == other || (self.is_numeric() && other.is_numeric())
    }
}

//melhora as mensagens de erro 
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataType::String => "String",
            DataType::Int => "Int",
            DataType::Float => "Float",
            DataType::Boolean => "Boolean",
        };
        write!(f, "{name}")
    }
}

//metodso que estavam sendo chamados partes diferentes do código ( typechecker e interpretador)
//render transforma uma tabela em linhas de texto alinhadas, com cabeçalho e contagem de linhas.
impl Table {
    /// Posição de uma coluna pelo nome.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|column| column.name == name)
    }

    /// Coluna pelo nome.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.name == name)
    }

    pub fn render(&self) -> Vec<String> {
        let width = |s: &str| s.chars().count();

        let mut widths: Vec<usize> = self.columns.iter().map(|c| width(&c.name)).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(width(cell));
            }
        }

        let format_row = |cells: Vec<&str>| -> String {
            cells
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{}{}", cell, " ".repeat(widths[i] - width(cell))))
                .collect::<Vec<_>>()
                .join(" | ")
                .trim_end()
                .to_string()
        };

        let mut lines = Vec::new();
        lines.push(format_row(self.columns.iter().map(|c| c.name.as_str()).collect()));
        lines.push(
            widths
                .iter()
                .map(|w| "-".repeat(*w))
                .collect::<Vec<_>>()
                .join("-+-"),
        );
        for row in &self.rows {
            lines.push(format_row(row.iter().map(String::as_str).collect()));
        }

        let count = self.rows.len();
        lines.push(format!("({} {})", count, if count == 1 { "linha" } else { "linhas" }));

        lines
    }
}