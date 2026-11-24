use std::io::{self, Write};

use mini_sql::pipeline::run_pipeline_traced;
use mini_sql::table::Database;

fn main() {
    let stdin = io::stdin();
    let mut database = Database::new();

    // `.trace` liga/desliga a exibição das tabelas temporárias das CTEs.
    // É um comando do terminal, não do SQL: por isso não passa pela gramática.
    let mut trace = false;

    loop {
        print!("MiniSQL> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();

        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();

        // Encerra o terminal
        if input.eq_ignore_ascii_case("exit")
            || input.eq_ignore_ascii_case("quit")
        {
            break;
        }

        // Ignora entradas vazias
        if input.is_empty() {
            continue;
        }

        if input == ".trace" {
            trace = !trace;
            println!("Trace {}.", if trace { "ligado" } else { "desligado" });
            continue;
        }

        // O pipeline coordena o processamento do SQL
        match run_pipeline_traced(input, &mut database, trace) {
            Ok(output) => {
                for line in output {
                    println!("{line}");
                }
            }
            Err(error) => println!("{error}"),
        }
    }
}