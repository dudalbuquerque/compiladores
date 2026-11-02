//! Roda sobre a AST, ANTES da execução, e rejeita programas que são

//! - tabela inexistente (`FROM xyz` sem `LOAD` antes);
//! - coluna inexistente no SELECT ou no WHERE;
//! - comparação entre tipos incompatíveis (`idade = 'vinte'`);
//! - operador de ordem em tipo sem ordem (`ativo > TRUE`);
//! - lista do IN com tipos misturados (`idade IN (1, 'a')`);
//! - LOAD de arquivo que não é `.csv` ou cujo nome não vira um identificador.
//!
//! O ambiente de tipos é o esquema das tabelas já carregadas: nome da
//! tabela -> colunas e seus tipos (`table::Database`). Os tipos das colunas
//! são inferidos pelo `csv_loader` no momento do LOAD.


//ponto de entrada é o statement, que pode ser LOAD, DESCRIBE ou SELECT. Cada um tem sua função de checagem.

use crate::ast::{Condition, Describe, Expr, ExprIn, Load, Operator, Query, SelectList, Statement, Value};
use crate::csv_loader::table_name_from_path;
use crate::table::{DataType, Database, Table};

pub type TypeResult<T> = Result<T, String>;

/// Ponto de entrada: verifica um statement contra as tabelas carregadas.
pub fn check_statement(statement: &Statement, database: &Database) -> TypeResult<()> {
    match statement {
        Statement::Load(load) => check_load(load),
        Statement::Describe(describe) => check_describe(describe, database),
        Statement::Query(query) => check_query(query, database),
    }
}

/// LOAD: o arquivo precisa ser `.csv` e o nome precisa virar um identificador.
/// A existência do arquivo só é verificada na execução 
pub fn check_load(load: &Load) -> TypeResult<()> {
    if !load.file.to_ascii_lowercase().ends_with(".csv") {
        return Err(format!("LOAD espera um arquivo .csv, recebeu '{}'", load.file));
    }
    table_name_from_path(&load.file)?;
    Ok(())
}

/// DESCRIBE: a tabela precisa existir.
pub fn check_describe(describe: &Describe, database: &Database) -> TypeResult<()> {
    lookup_table(&describe.table, database)?;
    Ok(())
}

/// Query: a tabela existe, as colunas do SELECT existem e o WHERE é bem tipado.
pub fn check_query(query: &Query, database: &Database) -> TypeResult<()> {
    let table = lookup_table(&query.table, database)?;

    if let SelectList::Columns(names) = &query.select {
        for name in names {
            lookup_column(name, &query.table, table)?;
        }
    }

    if let Some(condition) = &query.condition {
        check_condition(condition, &query.table, table)?;
    }

    Ok(())
}

/// Verifica recursivamente uma condição do WHERE.
pub fn check_condition(condition: &Condition, table_name: &str, table: &Table) -> TypeResult<()> {
    match condition {
        Condition::Expr(expr) => check_expr(expr, table_name, table),
        Condition::ExprIn(expr_in) => check_expr_in(expr_in, table_name, table),

        Condition::And(left, right) | Condition::Or(left, right) => {
            check_condition(left, table_name, table)?;
            check_condition(right, table_name, table)
        }

        Condition::Not(inner) | Condition::Parens(inner) => check_condition(inner, table_name, table),
    }
}

fn check_expr(expr: &Expr, table_name: &str, table: &Table) -> TypeResult<()> {
    let left_type = type_of(&expr.left, table_name, table)?;
    let right_type = type_of(&expr.right, table_name, table)?;

    if !left_type.is_compatible_with(&right_type) {
        return Err(format!(
            "Tipos incompatíveis em `{expr}`: {} é {left_type} e {} é {right_type}",
            describe_value(&expr.left),
            describe_value(&expr.right),
        ));
    }

    let is_order_op = matches!(
        expr.op,
        Operator::Less | Operator::LessEqual | Operator::Greater | Operator::GreaterEqual
    );

    if is_order_op && !left_type.is_ordered() {
        return Err(format!(
            "Operador `{}` não se aplica ao tipo {left_type} em `{expr}` (use = ou !=)",
            expr.op
        ));
    }

    Ok(())
}

/// Pertinência `value IN (v1, v2, ...)`: cada item segue a regra do `=`.
fn check_expr_in(expr_in: &ExprIn, table_name: &str, table: &Table) -> TypeResult<()> {
    let value_type = type_of(&expr_in.value, table_name, table)?;

    for item in &expr_in.values {
        let item_type = type_of(item, table_name, table)?;

        if !value_type.is_compatible_with(&item_type) {
            return Err(format!(
                "Tipos incompatíveis no IN: {} é {value_type}, mas o item {item} é {item_type}",
                describe_value(&expr_in.value),
            ));
        }
    }

    Ok(())
}

/// Tipo de um valor: literal tem tipo fixo; identificador tem o tipo da coluna.
fn type_of(value: &Value, table_name: &str, table: &Table) -> TypeResult<DataType> {
    match value {
        Value::Id(name) => Ok(lookup_column(name, table_name, table)?.clone()),
        Value::Int(_) => Ok(DataType::Int),
        Value::Float(_) => Ok(DataType::Float),
        Value::String(_) => Ok(DataType::String),
        Value::Boolean(_) => Ok(DataType::Boolean),
    }
}

fn lookup_table<'a>(name: &str, database: &'a Database) -> TypeResult<&'a Table> {
    database.get(name).ok_or_else(|| {
        format!("Tabela '{name}' não encontrada (use LOAD 'arquivo.csv'; antes de consultá-la)")
    })
}

fn lookup_column<'a>(name: &str, table_name: &str, table: &'a Table) -> TypeResult<&'a DataType> {
    table
        .column(name)
        .map(|column| &column.data_type)
        .ok_or_else(|| {
            let available: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
            format!(
                "Coluna '{name}' não existe na tabela '{table_name}' (colunas: {})",
                available.join(", ")
            )
        })
}

/// "a coluna 'idade'" ou "o literal 'vinte'", para as mensagens de erro.
fn describe_value(value: &Value) -> String {
    match value {
        Value::Id(name) => format!("a coluna '{name}'"),
        literal => format!("o literal {literal}"),
    }
}