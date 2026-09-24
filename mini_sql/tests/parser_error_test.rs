use antlr4_runtime::parser::Parser; // <--- Adicione esta linha (ou `use antlr4_runtime::Parser;`)
use antlr4_runtime::{CommonTokenStream, InputStream};

use mini_sql::mini_sql_lexer::MiniSqlLexer;
use mini_sql::mini_sql_parser::MiniSqlParser;

fn parse_error(input: &str) -> bool {
    let input = InputStream::new(input);
    let lexer = MiniSqlLexer::new(input);
    let tokens = CommonTokenStream::new(lexer);
    let mut parser = MiniSqlParser::new(tokens);

    //parser.program().is_err()
    // Executa o parser
    let result = parser.program();

    // Se houver algum erro de sintaxe registrado pelo parser ou se a função retornar Err
    result.is_err() || parser.number_of_syntax_errors() > 0
}



// users
// name age type credit active




/// syntax error tests
#[test]
fn test_missing_select_list() {
    assert!(parse_error("FROM users SELECT;"));
}

#[test]
fn test_missing_select_list_minus() {
    assert!(parse_error("from users select;"));
}


#[test]
fn test_missing_end() {
    assert!(parse_error("FROM users SELECT name"));
}

#[test]
fn test_missing_end_minus() {
    assert!(parse_error("from users select name"));
}

#[test]
fn test_missing_select() {
    assert!(parse_error("FROM users name;"));
}
#[test]
fn test_missing_select_minus() {
    assert!(parse_error("from users name;"));
}

#[test]
fn test_missing_from() {
    assert!(parse_error("SELECT name;"));
}

#[test]
fn test_missing_from_minus() {
    assert!(parse_error("select name;"));
}

#[test]
fn test_missing_where_condition() {
    assert!(parse_error("FROM users SELECT name WHERE;"));
}

#[test]
fn test_missing_where_condition_minus() {
    assert!(parse_error("from users select name where;"));
}


#[test]
fn test_trailing_comma_in_select() {
    assert!(parse_error("FROM users SELECT name, age, WHERE active = FALSE;"));
}


#[test]
fn test_trailing_comma_in_select_minus() {
    assert!(parse_error("from users select name, age, where active = false;"));
}


#[test]
fn test_empty_in_clause() {
    assert!(parse_error("FROM users SELECT name WHERE age IN ();"));
}

#[test]
fn test_empty_in_clause_minus() {
    assert!(parse_error("from users select name where age in ();"));
}


#[test]
fn test_unclosed_string_literal() {
    assert!(parse_error("FROM users SELECT name WHERE type = 'admin;"));
}
#[test]
fn test_unclosed_string_literal_minus() {
    assert!(parse_error("from users select name where type = 'admin;"));
}

#[test]
fn test_invalid_operator() {
    // Operadores como === o nã foram definidos na gramática
    assert!(parse_error("FROM users SELECT name WHERE age === 18;"));
}
#[test]
fn test_invalid_operator_minus() {
    // Operadores como ===  foram definidos na gramática
    assert!(parse_error("from users select name where age === 18;"));
}

////////// typo tests
#[test]
fn test_typo_in_from() {
    assert!(parse_error(
        "FORM users SELECT name WHERE age > 18 AND active = TRUE;"
    ));
}

#[test]
fn test_typo_in_from_minus() {
    assert!(parse_error(
        "form users select name where age > 18 and active = true;"
    ));
}

#[test]
fn test_typo_in_select() {
    assert!(parse_error("FROM users SELEC *;"));
}

#[test]
fn test_typo_in_select_minus() {
    assert!(parse_error("from users selec *;"));
}


////////// identifier tests

#[test]
fn test_invalid_table_syntax() {
    assert!(parse_error(
        "FROM users()_uugf SELECT name WHERE age > 18 OR active = TRUE;"
    ));
}

#[test]
fn test_invalid_table_syntax_minus() {
    assert!(parse_error(
        "from users()_uugf select name where age > 18 or active = true;"
    ));
}

#[test]
fn test_identifier_start_with_number() {
    assert!(parse_error("FROM 123users SELECT name;"));
}

#[test]
fn test_identifier_start_with_number_minus() {
    assert!(parse_error("from 123users select name;"));
}


//comentar com professor
#[test]
fn test_invalid_identifier_start() {
    assert!(parse_error("FROM _users SELECT name;"));
}

#[test]
fn test_invalid_identifier_start_minus() {
    assert!(parse_error("from _users select name;"));
}

#[test]
fn test_uppercase_identifier() {
    // O lexer só aceita identificadores começando com minúscula: [a-z] [a-z0-9_]*
    assert!(parse_error("FROM Users SELECT name;"));
}

#[test]
fn test_uppercase_identifier_minus() {
    // O lexer só aceita identificadores começando com minúscula: [a-z] [a-z0-9_]*
    assert!(parse_error("from Users select name;"));
}


////////// old syntax tests
#[test]
fn test_old_select_syntax() {
    assert!(parse_error("SELECT name FROM users;"));
}

#[test]
fn test_old_select_syntax_minus() {
    assert!(parse_error("select name from users;"));
}