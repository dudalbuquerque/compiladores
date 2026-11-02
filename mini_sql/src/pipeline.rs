// texto SQL --(lexer + parser + AstBuilder)--> AST
//           --(type checker)-----------------> AST verificada
//           --(interpretador)----------------> linhas de saída
//
// Cada statement é verificado e executado antes do próximo. Isso é
// necessário porque o esquema de uma tabela só é conhecido depois que o
// LOAD dela roda: em `LOAD 'alunos.csv'; FROM alunos SELECT nome;` o
// type checker só consegue verificar a query depois que o LOAD executou.

use crate::ast::Program;
use crate::ast_builder::AstBuilder;
use crate::interpreter::execute_statement;
use crate::table::Database;
use crate::type_checker::check_statement;

pub fn run_pipeline(sql: &str, database: &mut Database) -> Result<Vec<String>, String> {
    let program: Program = AstBuilder::parse_sql(sql).map_err(|e| format!("Erro de sintaxe: {e}"))?;

    execute_program(&program, database)
}

/// Para no primeiro erro (semântico ou de execução).
pub fn execute_program(program: &Program, database: &mut Database) -> Result<Vec<String>, String> {
    let mut output = Vec::new();

    for statement in &program.statements {
        // Análise semântica.
        check_statement(statement, database).map_err(|e| format!("Erro semântico: {e}"))?;

        // Execução.
        let lines = execute_statement(statement, database).map_err(|e| format!("Erro de execução: {e}"))?;
        output.extend(lines);
    }

    Ok(output)
}