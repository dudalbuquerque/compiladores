// texto SQL --(lexer + parser + AstBuilder)--> AST
//           --(type checker)-----------------> AST verificada
//           --(interpretador)----------------> linhas de saída
//
// Cada statement é verificado e executado antes do próximo. Isso é
// necessário porque o esquema de uma tabela só é conhecido depois que o
// LOAD dela roda: em `LOAD 'test_data/alunos.csv'; FROM alunos SELECT nome;` o
// type checker só consegue verificar a query depois que o LOAD executou.

use crate::ast::Program;
use crate::ast_builder::AstBuilder;
use crate::interpreter::execute_statement_traced;
use crate::table::Database;
use crate::type_checker::check_statement;

pub fn run_pipeline(sql: &str, database: &mut Database) -> Result<Vec<String>, String> {
    run_pipeline_traced(sql, database, false)
}

/// Igual a `run_pipeline`; com `trace = true`, a saída também mostra as
/// tabelas temporárias das CTEs sendo criadas e descartadas (`.trace` no terminal).
pub fn run_pipeline_traced(sql: &str, database: &mut Database, trace: bool) -> Result<Vec<String>, String> {
    let program: Program = AstBuilder::parse_sql(sql).map_err(|e| format!("Erro de sintaxe: {e}"))?;

    execute_program_traced(&program, database, trace)
}

/// Para no primeiro erro (semântico ou de execução).
pub fn execute_program(program: &Program, database: &mut Database) -> Result<Vec<String>, String> {
    execute_program_traced(program, database, false)
}

pub fn execute_program_traced(program: &Program, database: &mut Database, trace: bool) -> Result<Vec<String>, String> {
    let mut output = Vec::new();

    for statement in &program.statements {
        // Análise semântica.
        check_statement(statement, database).map_err(|e| format!("Erro semântico: {e}"))?;

        // Execução.
        let lines = execute_statement_traced(statement, database, trace).map_err(|e| format!("Erro de execução: {e}"))?;
        output.extend(lines);
    }

    Ok(output)
}