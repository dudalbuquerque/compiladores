
use crate::ast::{Program, Statement};
use crate::ast_builder::AstBuilder;
use crate::executor::{
    execute_describe,
    execute_load,
    execute_query,
};
use crate::table::Database;
use crate::type_checker::check_condition;

pub fn run_pipeline(
    sql: &str,
    database: &mut Database,
) -> Result<Vec<String>, String> {
    // Análise sintática: SQL -> AST.
    let program: Program = AstBuilder::parse_sql(sql)?;

    // Validação semântica e execução.
    execute_pipeline(&program, database)
}

fn execute_pipeline(
    program: &Program,
    database: &mut Database,
) -> Result<Vec<String>, String> {
    let mut output = Vec::new();

    for statement in &program.statements {
        match statement {
            Statement::Load(load) => {
                output.push(execute_load(load, database)?);
            }

            Statement::Query(query) => {
                let table = database
                    .get(&query.table)
                    .ok_or_else(|| {
                        format!("Tabela '{}' não encontrada", query.table)
                    })?;

                if let Some(condition) = &query.condition {
                    check_condition(condition, table)?;
                }

                let result = execute_query(query, database)?;

                for row in result.rows {
                    output.push(row.join(" | "));
                }
            }

            Statement::Describe(describe) => {
                output.extend(execute_describe(describe, database)?);
            }
        }
    }

    Ok(output)
}
