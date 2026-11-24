use antlr4_runtime::parser::Parser;
use antlr4_runtime::{CommonTokenStream, InputStream};

use mini_sql::mini_sql_lexer::MiniSqlLexer;
use mini_sql::mini_sql_parser::MiniSqlParser;

fn parse(input: &str) -> bool {
    let input = InputStream::new(input);
    let lexer = MiniSqlLexer::new(input);
    let tokens = CommonTokenStream::new(lexer);
    let mut parser = MiniSqlParser::new(tokens);

    // `program()` devolve Ok mesmo quando houve erro de sintaxe: o parser do
    // ANTLR se recupera (insere/descarta tokens) e segue. Por isso também
    // exigimos zero erros registrados; sem isso, este teste nunca falharia
    // (`FROM users SELECT;` dava Ok).
    let ok = parser.program().is_ok();
    ok && parser.number_of_syntax_errors() == 0
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
    assert!(parse("FROM users SELECT age WHERE credit >= -10.5;"));
}


#[test]
fn test_negative_numbers_minus() {
    assert!(parse("from users select age where credit >= -10.5;"));
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

#[test]
fn test_load_csv() {
    assert!(parse("LOAD 'users.csv';"));
}

#[test]
fn test_describe_table() {
    assert!(parse("DESCRIBE users;"));
}

#[test]
fn test_multiple_statement_types() {
    let input = "
        LOAD 'users.csv';
        DESCRIBE users;
        FROM users SELECT name;
    ";
    assert!(parse(input));
}

// CTEs (WITH)

#[test]
fn test_cte_simples() {
    assert!(parse("WITH adultos AS (FROM users SELECT name WHERE age >= 18) FROM adultos SELECT name;"));
}

#[test]
fn test_cte_simples_minus() {
    assert!(parse("with adultos as (from users select name where age >= 18) from adultos select name;"));
}

#[test]
fn test_cte_com_lista_de_colunas() {
    assert!(parse("WITH adultos (nome, anos) AS (FROM users SELECT name, age) FROM adultos SELECT nome;"));
}

#[test]
fn test_cte_com_lista_de_colunas_minus() {
    assert!(parse("with adultos (nome, anos) as (from users select name, age) from adultos select nome;"));
}

#[test]
fn test_varias_ctes() {
    assert!(parse("WITH a AS (FROM users SELECT name), b AS (FROM a SELECT name) FROM b SELECT name;"));
}

#[test]
fn test_varias_ctes_minus() {
    assert!(parse("with a as (from users select name), b as (from a select name) from b select name;"));
}

#[test]
fn test_cte_com_join() {
    assert!(parse("WITH c AS (FROM usuarios JOIN pedidos ON usuarios.id = usuario_id SELECT *) FROM c SELECT nome;"));
}

#[test]
fn test_cte_com_join_minus() {
    assert!(parse("with c as (from usuarios join pedidos on usuarios.id = usuario_id select *) from c select nome;"));
}

// Nomes qualificados (tabela.coluna)

#[test]
fn test_coluna_qualificada() {
    assert!(parse("FROM usuarios JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT usuarios.nome WHERE pedidos.valor > 100;"));
}

#[test]
fn test_coluna_qualificada_minus() {
    assert!(parse("from usuarios join pedidos on usuarios.id = pedidos.usuario_id select usuarios.nome where pedidos.valor > 100;"));
}

#[test]
fn test_coluna_qualificada_no_in() {
    assert!(parse("FROM users SELECT name WHERE users.type IN ('admin', 'editor');"));
}

// RIGHT e FULL JOIN (com e sem OUTER)

#[test]
fn test_right_join() {
    assert!(parse("FROM usuarios RIGHT JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto;"));
}

#[test]
fn test_right_outer_join_minus() {
    assert!(parse("from usuarios right outer join pedidos on usuarios.id = usuario_id select nome, produto;"));
}

#[test]
fn test_full_join() {
    assert!(parse("FROM usuarios FULL JOIN pedidos ON usuarios.id = usuario_id SELECT *;"));
}

#[test]
fn test_full_outer_join_minus() {
    assert!(parse("from usuarios full outer join pedidos on usuarios.id = usuario_id select *;"));
}

#[test]
fn test_left_outer_join() {
    assert!(parse("FROM usuarios LEFT OUTER JOIN pedidos ON usuarios.id = usuario_id SELECT nome;"));
}

#[test]
fn test_joins_encadeados_de_tipos_diferentes() {
    assert!(parse("FROM a JOIN b ON a.x = b.x LEFT JOIN c ON c.y = b.y RIGHT JOIN d ON d.z = c.z FULL JOIN e ON e.w = d.w SELECT *;"));
}