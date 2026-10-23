use std::io::{self, Write};
use mini_sql::ast_builder::AstBuilder;

fn main() {
    let stdin = io::stdin();

    loop {
        print!("MiniSQL> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();

        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            break;
        }

        if input.is_empty() {
            continue;
        }

        match AstBuilder::parse_sql(input) {
            Ok(ast) => println!("{:#?}", ast),
            Err(error) => println!("Erro: {}", error),
        }
    }
}