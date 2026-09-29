use antlr4_runtime::{CommonTokenStream, InputStream};

use mini_sql::mini_sql_lexer::MiniSqlLexer;
use mini_sql::mini_sql_parser::MiniSqlParser;

fn parse(input: &str) -> bool {
    let input = InputStream::new(input);
    let lexer = MiniSqlLexer::new(input);
    let tokens = CommonTokenStream::new(lexer);
    let mut parser = MiniSqlParser::new(tokens);

    parser.program().is_ok()
}

// users
// name age type credit active
#[test]
fn test_select_all() {
    assert!(parse("FROM users SELECT *;"));
}

#[test]
fn test_select_all_minus() {
    assert!(parse("from users select *;"));
}

#[test]
fn test_select_columns() {
    assert!(parse("FROM users SELECT name, age;"));
}

#[test]
fn test_select_columns_minus() {
    assert!(parse("from users select name, age;"));
}

#[test]
fn test_select_with_where() {
    assert!(parse("FROM users SELECT name WHERE age > 18;"));
}

#[test]
fn test_from_users_where_minus() {
    assert!(parse("from users select name where age > 18;"));
}

#[test]
fn test_select_with_and() {
    assert!(parse(
        "FROM users SELECT name WHERE age > 18 AND active = TRUE;"
    ));
}

#[test]
fn test_select_with_and_minus() {
    assert!(parse(
        "from users select name where age > 18 and active = true;"
    ));
}


#[test]
fn test_select_with_or() {
    assert!(parse(
        "FROM users SELECT name WHERE age > 18 OR active = TRUE;"
    ));
}
#[test]
fn test_select_with_or_minus() {
    assert!(parse(
        "from users select name where age > 18 or active = true;"
    ));
}

#[test]
fn test_select_with_in() {
    assert!(parse(
        "FROM users SELECT name WHERE age IN (18, 20, 22);"
    ));
}

#[test]
fn test_select_with_in_minus() {
    assert!(parse(
        "from users select name where age in (18, 20, 22);"
    ));
}


#[test]
fn test_select_in_with_strings() {
    assert!(parse("FROM users SELECT name WHERE type IN ('admin', 'editor', 'premium');"));
}

#[test]
fn test_select_in_with_strings_minus() {
    assert!(parse("from users select name where type in ('admin', 'editor', 'premium');"));
}


#[test]
fn test_select_string_and_float() {
    assert!(parse("FROM users SELECT name , age WHERE credit <= 9.99 AND type = 'premium';"));
}

#[test]
fn test_select_string_and_float_minus() {
    assert!(parse("from users select name , age where credit <= 9.99 and type = 'premium';"));
}


#[test]
fn test_negative_numbers() {
    assert!(parse("FROM users SELECT age WHERE credit >= -10.5"));
}


#[test]
fn test_negative_numbers_minus() {
    assert!(parse("from users select age where credit >= -10.5"));
}

#[test]
fn test_select_with_not_and_parenthesis() {
    assert!(parse("FROM users SELECT name WHERE NOT (age < 18 OR active = FALSE);"));
}

#[test]
fn test_select_with_not_and_parenthesis_minus() {
    assert!(parse("from users select name where not (age < 18 or active = false);"));
}


#[test]
fn test_select_with_not_equal_alt() {
    assert!(parse("FROM users SELECT name WHERE credit <> 0;"));
}


#[test]
fn test_select_with_not_equal_alt_minus() {
    assert!(parse("from users select name where credit <> 0;"));
}



#[test]
fn test_multiple_queries_in_program() {
    // Testa se o parser aceita mais de uma consulta (query+)
    let input = "
        FROM users SELECT name WHERE id = 1;
        FROM users SELECT type WHERE credit <= 100;
    ";
    assert!(parse(input));
}


#[test]
fn test_multiple_queries_in_program_minus() {
    // Testa se o parser aceita mais de uma consulta (query+)
    let input = "
        from users select name where id = 1;
        from users select type where credit <= 100;
    ";
    assert!(parse(input));
}

#[test]
fn test_comments_and_whitespaces() {
    let input = "
        -- Este e um comentario sobre a query
        FROM users -- comentario inline
        SELECT name, credit
        where type = 'editor';
    ";
    assert!(parse(input));
}

#[test]
fn test_comments_and_whitespaces_minus() {
    let input = "
        -- Este e um comentario sobre a query
        from users -- comentario inline
        select name,credit 
        where type = 'editor';
    ";
    assert!(parse(input));
}

