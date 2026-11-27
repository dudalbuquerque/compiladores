use mini_sql::ast::*;
use mini_sql::ast_builder::AstBuilder;

#[test]
fn test_ast_generation() {
    let input = "FROM users SELECT name, credit WHERE type IN ('admin', 'editor') AND active = TRUE AND credit >= 150.50;";

    let ast = AstBuilder::parse_sql(input).expect("Falha ao construir a AST");

    
    // O {:#?} imprime a struct/enum formatada com indentacao
    println!("\n=== ÁRVORE AST GERADA ===");
    println!("{:#?}", ast);
    println!("=========================\n");

    
    assert_eq!(ast.statements.len(), 1);
    let query = match &ast.statements[0] {
        Statement::Query(query) => query,
        _ => panic!("Esperava uma Statement::Query"),
    };

    assert!(query.ctes.is_empty(), "query sem WITH não deveria ter CTEs");
    let query = &query.body;

    assert_eq!(query.table, "users");

    // Valida a lista de seleções (colunas `name` e `credit`)
    match &query.select {
        SelectList::Columns(cols) => {
            assert_eq!(cols, &vec!["name".to_string(), "credit".to_string()]);
        }
        _ => panic!("Esperava SelectList::Columns, mas recebeu SelectList::All"),
    }

    //Valida a estrutura das condições no WHERE
    let condition = query.condition.as_ref().expect("Esperava uma clausula WHERE na query");

    // Valida a raiz da árvore de condições (AND)
    if let Condition::And(left_and, right_cond) = condition {

        // Lado direito do AND principal: `credit >= 150.50`
        if let Condition::Expr(expr) = &**right_cond {
            assert_eq!(expr.left, Value::Id("credit".to_string()));
            assert_eq!(expr.op, Operator::GreaterEqual);
            assert_eq!(expr.right, Value::Float(150.50));
        } else {
            panic!("Esperava uma expressao de comparacao no lado direito do AND principal");
        }

        // Lado esquerdo do AND principal: `type IN ('admin', 'editor') AND active = TRUE`
        if let Condition::And(in_cond, active_cond) = &**left_and {
            
            // Valida `type IN ('admin', 'editor')`
            if let Condition::ExprIn(expr_in) = &**in_cond {
                assert_eq!(expr_in.value, Value::Id("type".to_string()));
                assert_eq!(
                    expr_in.values,
                    vec![
                        Value::String("admin".to_string()),
                        Value::String("editor".to_string())
                    ]
                );
            } else {
                panic!("Esperava uma expressao IN para a coluna 'type'");
            }

            // Valida `active = TRUE`
            if let Condition::Expr(expr) = &**active_cond {
                assert_eq!(expr.left, Value::Id("active".to_string()));
                assert_eq!(expr.op, Operator::Equal);
                assert_eq!(expr.right, Value::Boolean(true));
            } else {
                panic!("Esperava a comparacao booleana para a coluna 'active'");
            }

        } else {
            panic!("Esperava um sub-AND no lado esquerdo da condicao");
        }

    } else {
        panic!("Esperava que o no raiz das condicoes fosse Condition::And");
    }
}

#[test]
fn test_load_statement() {
    let ast = AstBuilder::parse_sql("LOAD 'users.csv';")
        .expect("Falha ao construir AST do LOAD");
    
    println!("\n=== AST: LOAD ===");
    println!("{:#?}", ast);
    println!("=================\n");
    
    assert_eq!(
        ast.statements,
        vec![Statement::Load(Load {
            file: "users.csv".to_string()
        })]
    );
}

#[test]
fn test_describe_statement() {
    let ast = AstBuilder::parse_sql("DESCRIBE users;")
        .expect("Falha ao construir AST do DESCRIBE");

    println!("\n=== AST: DESCRIBE ===");
    println!("{:#?}", ast);
    println!("=================\n");

    assert_eq!(
        ast.statements,
        vec![Statement::Describe(Describe {
            table: "users".to_string()
        })]
    );
}

// ----- CTEs e nomes qualificados -----------------------------------------

fn only_query(sql: &str) -> Query {
    let ast = AstBuilder::parse_sql(sql).expect("Falha ao construir a AST");
    match ast.statements.into_iter().next() {
        Some(Statement::Query(query)) => query,
        other => panic!("Esperava uma Statement::Query, veio {other:?}"),
    }
}

#[test]
fn test_ast_cte_com_lista_de_colunas() {
    let query = only_query("WITH adultos (nome, anos) AS (FROM users SELECT name, age WHERE age >= 18) FROM adultos SELECT nome;");

    assert_eq!(query.ctes.len(), 1);
    let cte = &query.ctes[0];
    assert_eq!(cte.name, "adultos");
    assert_eq!(cte.columns, Some(vec!["nome".to_string(), "anos".to_string()]));
    assert_eq!(cte.body.table, "users");
    assert_eq!(cte.body.select, SelectList::Columns(vec!["name".into(), "age".into()]));
    assert!(cte.body.condition.is_some());

    // o corpo principal lê a CTE
    assert_eq!(query.body.table, "adultos");
    assert_eq!(query.body.condition, None);
}

#[test]
fn test_ast_varias_ctes_sem_lista() {
    let query = only_query("with a as (from users select name), b as (from a select name) from b select name;");

    let names: Vec<&str> = query.ctes.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
    assert!(query.ctes.iter().all(|c| c.columns.is_none()));
    assert_eq!(query.ctes[1].body.table, "a");
    assert_eq!(query.body.table, "b");
}

#[test]
fn test_ast_cte_com_join() {
    let query = only_query(
        "WITH c AS (FROM usuarios LEFT JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT nome) FROM c SELECT nome;",
    );

    let join = &query.ctes[0].body.joins[0];
    assert_eq!(join.join_type, JoinType::Left);
    assert_eq!(join.table, "pedidos");
}

#[test]
fn test_ast_coluna_qualificada() {
    let query = only_query(
        "FROM usuarios JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT usuarios.nome, produto WHERE pedidos.valor > 100;",
    );

    // o nome qualificado é guardado como foi escrito; a resolução é no type checker
    assert_eq!(query.body.joins[0].condition.left, Value::Id("usuarios.id".to_string()));
    assert_eq!(query.body.joins[0].condition.right, Value::Id("pedidos.usuario_id".to_string()));
    assert_eq!(query.body.select, SelectList::Columns(vec!["usuarios.nome".into(), "produto".into()]));
    match &query.body.condition {
        Some(Condition::Expr(expr)) => assert_eq!(expr.left, Value::Id("pedidos.valor".to_string())),
        other => panic!("Esperava Condition::Expr, veio {other:?}"),
    }
}

#[test]
fn test_ast_float_continua_sendo_float() {
    // `150.50` não pode virar `150` `.` `50` por causa do novo token DOT
    let query = only_query("FROM users SELECT name WHERE credit >= 150.50;");

    match &query.body.condition {
        Some(Condition::Expr(expr)) => assert_eq!(expr.right, Value::Float(150.50)),
        other => panic!("Esperava Condition::Expr, veio {other:?}"),
    }
}

#[test]
fn test_ast_nao_e_gerada_com_erro_de_sintaxe() {
    // O parser se recupera de `()` vazio inventando um ID ("<missing ID>");
    // parse_sql precisa recusar em vez de devolver essa AST.
    let err = AstBuilder::parse_sql("WITH a () AS (FROM users SELECT name) FROM a SELECT name;").unwrap_err();
    assert!(err.contains("missing ID"), "{err}");

    assert!(AstBuilder::parse_sql("FROM users SELECT name").is_err(), "sem ';' deveria ser erro");
}

#[test]
fn test_ast_tipos_de_join() {
    let query = only_query(
        "FROM a JOIN b ON a.x = b.x INNER JOIN c ON c.x = a.x LEFT OUTER JOIN d ON d.x = a.x \
         RIGHT JOIN e ON e.x = a.x FULL OUTER JOIN f ON f.x = a.x SELECT *;",
    );

    let tipos: Vec<JoinType> = query.body.joins.iter().map(|j| j.join_type.clone()).collect();
    assert_eq!(
        tipos,
        vec![JoinType::Inner, JoinType::Inner, JoinType::Left, JoinType::Right, JoinType::Full]
    );
    let tabelas: Vec<&str> = query.body.joins.iter().map(|j| j.table.as_str()).collect();
    assert_eq!(tabelas, vec!["b", "c", "d", "e", "f"]);
}