
use crate::ast::{Condition, Expr, ExprIn, Operator, Value};
use crate::table::{DataType, Table};

// verifica todos os tipos de condições do WHERE
pub fn check_condition(
    condition: &Condition,
    table: &Table,
) -> Result<(), String> {
    match condition {
        Condition::Expr(expr) => check_expr(expr, table),

        Condition::ExprIn(expr_in) => check_expr_in(expr_in, table),

        Condition::And(left, right) | Condition::Or(left, right) => {
            check_condition(left, table)?;
            check_condition(right, table)
        }

        Condition::Not(condition) | Condition::Parens(condition) => {
            check_condition(condition, table)
        }
    }
}

// verifica a compatibilidade dos tipos de uma comparação
fn check_expr(
    expr: &Expr,
    table: &Table,
) -> Result<(), String> {
    let left_type = get_value_type(&expr.left, table)?;
    let right_type = get_value_type(&expr.right, table)?;

    if left_type != right_type {
        return Err(format!(
            "Tipos incompatíveis: {:?} e {:?}",
            left_type, right_type
        ));
    }

    // Operadores de comparação precisam de tipos compatíveis
    // pode ser refinada depois
    match expr.op {
        Operator::Less
        | Operator::LessEqual
        | Operator::Greater
        | Operator::GreaterEqual => {
            match left_type {
                DataType::Int | DataType::Float | DataType::String => Ok(()),
                _ => Err(format!(
                    "Operador de ordenação não suportado para {:?}",
                    left_type
                )),
            }
        }

        Operator::Equal | Operator::NotEqual => Ok(()),
    }
}

// Verifica o valor usado com IN e os valores da lista
fn check_expr_in(
    expr_in: &ExprIn,
    table: &Table,
) -> Result<(), String> {
    let value_type = get_value_type(&expr_in.value, table)?;

    for value in &expr_in.values {
        let item_type = get_value_type(value, table)?;

        if value_type != item_type {
            return Err(format!(
                "Tipos incompatíveis no IN: {:?} e {:?}",
                value_type, item_type
            ));
        }
    }

    Ok(())
}

// Descobre o tipo de um valor ou de uma coluna
fn get_value_type(
    value: &Value,
    table: &Table,
) -> Result<DataType, String> {
    match value {
        Value::Id(name) => {
            let column = table.columns
                .iter()
                .find(|column| column.name == *name)
                .ok_or_else(|| {
                    format!("Coluna '{}' não encontrada", name)
                })?;

            Ok(column.data_type.clone())
        }

        Value::Int(_) => Ok(DataType::Int),
        Value::Float(_) => Ok(DataType::Float),
        Value::String(_) => Ok(DataType::String),
        Value::Boolean(_) => Ok(DataType::Boolean),
    }
}
