// Interpretador: executa a AST (já verificada pelo type checker) sobre
// as tabelas carregadas em memória.

// As células ficam guardadas como texto em `Table::rows`. Na hora de
// avaliar o WHERE, cada célula é convertida para um valor tipado
// (`Cell`) usando o tipo que o `csv_loader` inferiu para a coluna.

// Célula vazia no CSV vira `Cell::Null` e nao quebra a consulta
use std::cmp::Ordering;

use crate::ast::{Condition, Describe, Expr, ExprIn, Load, Operator, Query, SelectList, Statement, Value};
use crate::csv_loader::load_csv, table_name_from_path;
use crate::table::{Column, DataType, Database, Table};

pub type ExecResult<T> = Result<T, String>;

type Truth = Option<bool>;

#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
}

// Statements

pub fn execute_statement(statement: &Statement, database: &mut Database) -> ExecResult<Vec<String>> {
    match statement {
        Statement::Load(load) => Ok(vec![execute_load(load, database)?]),
        Statement::Describe(describe) => execute_describe(describe, database),
        Statement::Query(query) => Ok(execute_query(query, database)?.render()),
    }
}

/// LOAD 'arquivo.csv'; -> lê o CSV e registra a tabela com o nome do arquivo.
pub fn execute_load(load: &Load, database: &mut Database) -> ExecResult<String> {
    let name = table_name_from_path(&load.file)?;

    let table = load_csv(&load.file).map_err(|error| format!("Erro ao carregar '{}': {error}", load.file))?;

    let replaced = database.insert(name.clone(), table).is_some();

    Ok(format!(
        "Tabela '{name}' {}.",
        if replaced { "recarregada" } else { "carregada" },
    ))
}

/// DESCRIBE tabela; -> lista as colunas e seus tipos.
pub fn execute_describe(describe: &Describe, database: &Database) -> ExecResult<Vec<String>> {
    let table = get_table(&describe.table, database)?;

    let schema = Table {
        columns: vec![
            Column { name: "coluna".to_string(), data_type: DataType::String },
            Column { name: "tipo".to_string(), data_type: DataType::String },
        ],
        rows: table
            .columns
            .iter()
            .map(|column| vec![column.name.clone(), column.data_type.to_string()])
            .collect(),
    };

    let mut lines = schema.render();
    lines.pop(); // remove o "(n linhas)": aqui ele contaria colunas, não linhas
    lines.push(format!("({} colunas, {} linhas)", table.columns.len(), table.rows.len()));
    Ok(lines)
}

/// FROM tabela SELECT ... WHERE ...; -> devolve uma nova tabela com o resultado.
pub fn execute_query(query: &Query, database: &Database) -> ExecResult<Table> {
    let table = get_table(&query.table, database)?;

    // Projeção: índices das colunas pedidas no SELECT.
    let selected: Vec<usize> = match &query.select {
        SelectList::All => (0..table.columns.len()).collect(),
        SelectList::Columns(names) => names
            .iter()
            .map(|name| column_index(name, table))
            .collect::<ExecResult<_>>()?,
    };

    // Seleção: mantém só as linhas em que o WHERE é verdadeiro.
    let mut rows = Vec::new();
    for row in &table.rows {
        let keep = match &query.condition {
            Some(condition) => evaluate_condition(condition, row, table)? == Some(true),
            None => true,
        };

        if keep {
            rows.push(selected.iter().map(|&i| row[i].clone()).collect());
        }
    }

    let columns = selected.iter().map(|&i| table.columns[i].clone()).collect();

    Ok(Table { columns, rows })
}

// Avaliação do WHERE

/// Avalia uma condição para uma linha.
fn evaluate_condition(condition: &Condition, row: &[String], table: &Table) -> ExecResult<Truth> {
    Ok(match condition {
        Condition::Expr(expr) => evaluate_expr(expr, row, table)?,
        Condition::ExprIn(expr_in) => evaluate_in(expr_in, row, table)?,
        Condition::Parens(inner) => evaluate_condition(inner, row, table)?,
        Condition::Not(inner) => evaluate_condition(inner, row, table)?.map(|b| !b),

        // AND: falso se algum lado é falso; senão desconhecido se algum é desconhecido.
        Condition::And(left, right) => {
            match (evaluate_condition(left, row, table)?, evaluate_condition(right, row, table)?) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            }
        }

        // OR: verdadeiro se algum lado é verdadeiro; senão desconhecido se algum é desconhecido.
        Condition::Or(left, right) => {
            match (evaluate_condition(left, row, table)?, evaluate_condition(right, row, table)?) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            }
        }
    })
}

/// `left op right`.
fn evaluate_expr(expr: &Expr, row: &[String], table: &Table) -> ExecResult<Truth> {
    let left = evaluate_value(&expr.left, row, table)?;
    let right = evaluate_value(&expr.right, row, table)?;
    compare(&left, &expr.op, &right).map_err(|e| format!("{e} em `{expr}`"))
}

/// `value IN (v1, v2, ...)`: equivale a `value = v1 OR value = v2 OR ...`.
fn evaluate_in(expr_in: &ExprIn, row: &[String], table: &Table) -> ExecResult<Truth> {
    let value = evaluate_value(&expr_in.value, row, table)?;

    let mut result = Some(false);
    for item in &expr_in.values {
        let item = evaluate_value(item, row, table)?;
        match compare(&value, &Operator::Equal, &item)? {
            Some(true) => return Ok(Some(true)),
            Some(false) => {}
            None => result = None,
        }
    }
    Ok(result)
}

//mais tipos, antes só int64
fn compare(left: &Cell, op: &Operator, right: &Cell) -> ExecResult<Truth> {
    let ordering = match (left, right) {
        (Cell::Null, _) | (_, Cell::Null) => return Ok(None),

        (Cell::Int(a), Cell::Int(b)) => a.cmp(b),
        (Cell::Int(a), Cell::Float(b)) => cmp_float(*a as f64, *b)?,
        (Cell::Float(a), Cell::Int(b)) => cmp_float(*a, *b as f64)?,
        (Cell::Float(a), Cell::Float(b)) => cmp_float(*a, *b)?,
        (Cell::Str(a), Cell::Str(b)) => a.cmp(b),

        (Cell::Bool(a), Cell::Bool(b)) => {
            return match op {
                Operator::Equal => Ok(Some(a == b)),
                Operator::NotEqual => Ok(Some(a != b)),
                _ => Err(format!("Operador `{op}` não se aplica a Boolean")),
            };
        }

        // O type checker impede que isso aconteça; fica como proteção.
        _ => return Err(format!("Não é possível comparar {left:?} com {right:?}")),
    };

    Ok(Some(match op {
        Operator::Equal => ordering == Ordering::Equal,
        Operator::NotEqual => ordering != Ordering::Equal,
        Operator::Less => ordering == Ordering::Less,
        Operator::LessEqual => ordering != Ordering::Greater,
        Operator::Greater => ordering == Ordering::Greater,
        Operator::GreaterEqual => ordering != Ordering::Less,
    }))
}

fn cmp_float(a: f64, b: f64) -> ExecResult<Ordering> {
    a.partial_cmp(&b).ok_or_else(|| "Comparação com NaN".to_string())
}

/// Valor de um literal, ou da célula da linha atual se for uma coluna.
fn evaluate_value(value: &Value, row: &[String], table: &Table) -> ExecResult<Cell> {
    Ok(match value {
        Value::Int(v) => Cell::Int(*v),
        Value::Float(v) => Cell::Float(*v),
        Value::String(v) => Cell::Str(v.clone()),
        Value::Boolean(v) => Cell::Bool(*v),
        Value::Id(name) => {
            let index = column_index(name, table)?;
            parse_cell(&row[index], &table.columns[index])?
        }
    })
}

/// Converte o texto de uma célula para o tipo da coluna.
fn parse_cell(raw: &str, column: &Column) -> ExecResult<Cell> {
    if raw.is_empty() {
        return Ok(Cell::Null);
    }

    let invalid = || format!("Valor '{raw}' inválido para a coluna '{}' ({})", column.name, column.data_type);

    Ok(match column.data_type {
        DataType::Int => Cell::Int(raw.parse().map_err(|_| invalid())?),
        DataType::Float => Cell::Float(raw.parse().map_err(|_| invalid())?),
        DataType::Boolean => Cell::Bool(raw.eq_ignore_ascii_case("true")),
        DataType::String => Cell::Str(raw.to_string()),
    })
}

// Auxiliares

fn get_table<'a>(name: &str, database: &'a Database) -> ExecResult<&'a Table> {
    database.get(name).ok_or_else(|| format!("Tabela '{name}' não encontrada"))
}

fn column_index(name: &str, table: &Table) -> ExecResult<usize> {
    table.column_index(name).ok_or_else(|| format!("Coluna '{name}' não encontrada"))
}