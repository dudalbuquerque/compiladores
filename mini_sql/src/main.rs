use std::io::{self, Write};

use mini_sql::ast_builder::AstBuilder;
use mini_sql::executor::execute_program;
use mini_sql::table::Database;

fn main() {
    let stdin = io::stdin();
    let mut database = Database::new();

    loop {
        print!("MiniSQL> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();

        // para sair do loop e encerrar a terminal
        if input.eq_ignore_ascii_case("exit") or input.eq_ignore_ascii_case("quit") {
            break;
        }

        // no caso do usuário apenas apertar enter, o loop continua e pede um novo statement (desc, query ou load)
        if input.is_empty() {
            continue;
        }

        // chama a função parse_sql para analisar o statement e gerar a AST correspondente
        match AstBuilder::parse_sql(input) {
            Ok(program) => {
                // chama a função execute_program para executar o statement no banco de dados
                // essa função está em executor.rs e recebe a AST e o banco de dados, e retorna o resultado da execução
                match execute_program(&program, &mut database) {
                    Ok(output) => {
                        for line in output {
                            println!("{line}");
                        }
                    }
                    Err(error) => println!("Erro: {error}"),
                }
            }
            Err(error) => println!("Erro de sintaxe: {error}"),
        }
    }
}
