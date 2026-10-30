use crate::ast::{Condition, Describe, Expr, Load, Operator, Query, SelectList, Value, };
use crate::csv_loader::load_csv;
use crate::table::{Database, Table};

// avaliar uma condição do WHERE e retornar se ela é verdadeira (true) ou falsa (false) para uma linha da tabela
fn evaluate_condition(condition: &Condition,row: &[String], table: &Table,) -> Result<bool, String> {
    match condition {
        Condition::Expr(expr) => evaluate_expr(expr, row, table),

        _ => Err("Condição ainda não implementada".to_string()),
    }
}

// Para FROM alunos SELECT nome WHERE idade = 20;, ela localiza a coluna idade, converte seu conteúdo para inteiro e compara com 20. 
// O resultado é true ou false.
fn evaluate_expr(expr: &Expr, row: &[String], table: &Table,) -> Result<bool, String> {
    let left = match &expr.left {
        Value::Id(name) => {
            let index = table.columns
                .iter()
                .position(|column| column.name == *name)
                .ok_or_else(|| format!("Coluna '{}' não encontrada", name))?;

            row[index]
                .parse::<i64>()
                .map_err(|_| format!("Valor da coluna '{}' não é inteiro", name))?
        }
        Value::Int(value) => *value,
        _ => return Err("Tipo de valor ainda não suportado".to_string()),
    };

    let right = match &expr.right {
        Value::Id(name) => {
            let index = table.columns
                .iter()
                .position(|column| column.name == *name)
                .ok_or_else(|| format!("Coluna '{}' não encontrada", name))?;

            row[index]
                .parse::<i64>()
                .map_err(|_| format!("Valor da coluna '{}' não é inteiro", name))?
        }
        Value::Int(value) => *value,
        _ => return Err("Tipo de valor ainda não suportado".to_string()),
    };

    Ok(match expr.op {
        Operator::Equal => left == right,
        Operator::NotEqual => left != right,
        Operator::Less => left < right,
        Operator::LessEqual => left <= right,
        Operator::Greater => left > right,
        Operator::GreaterEqual => left >= right,
    })
}

// server para executar a query no banco de dados, para não fazer na main.rs
pub fn execute_query(query: &Query, database: &Database) -> Result<Table, String> {
    // checa se a tabela está no banco ou seja se seu o load no csv
    let table = database
        .get(&query.table)
        .ok_or_else(|| format!("Tabela '{}' não encontrada", query.table))?;

    // seleciona as colunas da tabela de acordo com a query (no arquivo table.rs tem a função as structs que definem do database, tabelas e colunas)
    let selected_columns: Vec<usize> = match &query.select {
        SelectList::All => (0..table.columns.len()).collect(),

        SelectList::Columns(names) => names
            .iter()
            .map(|name| {
                table.columns
                    .iter()
                    .position(|column| column.name == *name)
                    .ok_or_else(|| format!("Coluna '{}' não encontrada", name))
            })
            .collect::<Result<Vec<_>, _>>()?,
    };

    let columns = selected_columns
        .iter()
        .map(|&index| table.columns[index].clone())
        .collect();

    let rows = table.rows
        .iter()
        .filter_map(|row| {
            let matches = match &query.condition {
                Some(condition) => evaluate_condition(condition, row, table),
                None => Ok(true),
            };

            match matches {
                Ok(true) => Some(Ok(
                    selected_columns
                        .iter()
                        .map(|&index| row[index].clone())
                        .collect::<Vec<_>>()
                )),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(Table { columns, rows })
}


pub fn execute_load(
    load: &Load,
    database: &mut Database,
) -> Result<String, String> {
    let table = load_csv(&load.file)
        .map_err(|error| format!("Erro ao carregar CSV: {error}"))?;

    let name = std::path::Path::new(&load.file)
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("Não foi possível obter o nome da tabela")?
        .to_string();

    database.insert(name.clone(), table);

    Ok(format!("Tabela '{name}' carregada."))
}

pub fn execute_describe(
    describe: &Describe,
    database: &Database,
) -> Result<Vec<String>, String> {
    let table = database
        .get(&describe.table)
        .ok_or_else(|| format!("Tabela '{}' não encontrada", describe.table))?;

    let output = table
        .columns
        .iter()
        .map(|column| format!("{}: {:?}", column.name, column.data_type))
        .collect();

    Ok(output)
}



