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

    
    assert_eq!(ast.queries.len(), 1);
    let query = &ast.queries[0];

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