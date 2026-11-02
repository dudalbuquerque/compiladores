// Testes do interpretador.

// test_data/users.csv:

use mini_sql::ast::*;
use mini_sql::interpreter::{execute_describe, execute_load, execute_query, execute_statement};
use mini_sql::table::{DataType, Database};

// ----- auxiliares --------------------------------------------------------

fn db() -> Database {
    let mut db = Database::new();
    execute_load(&Load { file: "test_data/users.csv".into() }, &mut db).expect("LOAD users.csv");
    db
}

fn id(name: &str) -> Value {
    Value::Id(name.to_string())
}

fn string(s: &str) -> Value {
    Value::String(s.to_string())
}

fn cmp(left: Value, op: Operator, right: Value) -> Condition {
    Condition::Expr(Expr { left, op, right })
}

fn and(a: Condition, b: Condition) -> Condition {
    Condition::And(Box::new(a), Box::new(b))
}

fn or(a: Condition, b: Condition) -> Condition {
    Condition::Or(Box::new(a), Box::new(b))
}

fn not(a: Condition) -> Condition {
    Condition::Not(Box::new(a))
}

/// Executa `FROM users SELECT name WHERE <condition>` e devolve os nomes.
fn names_where(condition: Condition) -> Vec<String> {
    let query = Query {
        table: "users".into(),
        select: SelectList::Columns(vec!["name".into()]),
        condition: Some(condition),
    };
    execute_query(&query, &db())
        .expect("execução")
        .rows
        .into_iter()
        .map(|row| row[0].clone())
        .collect()
}

// ----- LOAD e DESCRIBE ---------------------------------------------------

#[test]
fn load_registra_tabela_com_nome_do_arquivo() {
    let mut db = Database::new();
    let msg = execute_load(&Load { file: "test_data/alunos.csv".into() }, &mut db).unwrap();

    assert_eq!(msg, "Tabela 'alunos' carregada (3 linhas).");
    assert!(db.contains_key("alunos"));
}

#[test]
fn load_do_mesmo_arquivo_recarrega() {
    let mut db = db();
    let msg = execute_load(&Load { file: "test_data/users.csv".into() }, &mut db).unwrap();
    assert!(msg.contains("recarregada"), "{msg}");
}

#[test]
fn load_de_arquivo_inexistente() {
    let mut db = Database::new();
    let err = execute_load(&Load { file: "test_data/nao_existe.csv".into() }, &mut db).unwrap_err();
    assert!(err.contains("Erro ao carregar"), "{err}");
}

#[test]
fn tipos_inferidos_do_csv() {
    let db = db();
    let types: Vec<DataType> = db["users"].columns.iter().map(|c| c.data_type.clone()).collect();
    assert_eq!(
        types,
        vec![DataType::String, DataType::Int, DataType::String, DataType::Float, DataType::Boolean]
    );
}

#[test]
fn describe_lista_colunas_e_tipos() {
    let lines = execute_describe(&Describe { table: "users".into() }, &db()).unwrap();

    assert_eq!(lines[0], "coluna | tipo");
    assert!(lines.contains(&"age    | Int".to_string()), "{lines:#?}");
    assert!(lines.contains(&"credit | Float".to_string()), "{lines:#?}");
    assert_eq!(lines.last().unwrap(), "(5 colunas, 5 linhas)");
}

// ----- SELECT ------------------------------------------------------------

#[test]
fn select_todas_as_colunas() {
    let query = Query { table: "users".into(), select: SelectList::All, condition: None };
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(result.columns.len(), 5);
    assert_eq!(result.rows.len(), 5);
}

#[test]
fn select_projeta_na_ordem_pedida() {
    let query = Query {
        table: "users".into(),
        select: SelectList::Columns(vec!["credit".into(), "name".into()]),
        condition: None,
    };
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(result.columns[0].name, "credit");
    assert_eq!(result.rows[0], vec!["200.75", "ana"]);
}

#[test]
fn saida_formatada_em_tabela() {
    let query = Query {
        table: "users".into(),
        select: SelectList::Columns(vec!["name".into(), "age".into()]),
        condition: Some(cmp(id("age"), Operator::Less, Value::Int(20))),
    };
    let mut db = db();
    let lines = execute_statement(&Statement::Query(query), &mut db).unwrap();

    assert_eq!(lines, vec!["name  | age", "------+----", "bruno | 17", "(1 linha)"]);
}

// ----- WHERE: comparações ------------------------------------------------

#[test]
fn where_int() {
    assert_eq!(names_where(cmp(id("age"), Operator::GreaterEqual, Value::Int(28))), ["carla", "diego", "elisa"]);
    assert_eq!(names_where(cmp(id("age"), Operator::Equal, Value::Int(17))), ["bruno"]);
    assert_eq!(names_where(cmp(id("age"), Operator::NotEqual, Value::Int(17))).len(), 4);
}

#[test]
fn where_float() {
    assert_eq!(names_where(cmp(id("credit"), Operator::GreaterEqual, Value::Float(150.5))), ["ana", "bruno", "elisa"]);
}

#[test]
fn where_float_com_literal_int() {
    assert_eq!(names_where(cmp(id("credit"), Operator::Greater, Value::Int(200))), ["ana", "elisa"]);
}

#[test]
fn where_string() {
    assert_eq!(names_where(cmp(id("type"), Operator::Equal, string("admin"))), ["ana", "diego"]);
    // ordem lexicográfica
    assert_eq!(names_where(cmp(id("name"), Operator::Less, string("c"))), ["ana", "bruno"]);
}

#[test]
fn where_boolean() {
    assert_eq!(names_where(cmp(id("active"), Operator::Equal, Value::Boolean(false))), ["bruno"]);
}

#[test]
fn where_literal_a_esquerda() {
    assert_eq!(names_where(cmp(Value::Int(30), Operator::Less, id("age"))), ["carla", "diego"]);
}

#[test]
fn where_entre_duas_colunas() {
    // age < credit vale para todos com crédito preenchido; diego (credit vazio) fica de fora.
    assert_eq!(names_where(cmp(id("age"), Operator::Less, id("credit"))), ["ana", "bruno", "carla", "elisa"]);
}

// ----- WHERE: conectivos -------------------------------------------------

#[test]
fn where_and() {
    let c = and(cmp(id("type"), Operator::Equal, string("editor")), cmp(id("active"), Operator::Equal, Value::Boolean(true)));
    assert_eq!(names_where(c), ["elisa"]);
}

#[test]
fn where_or() {
    let c = or(cmp(id("age"), Operator::Less, Value::Int(18)), cmp(id("age"), Operator::Greater, Value::Int(40)));
    assert_eq!(names_where(c), ["bruno", "diego"]);
}

#[test]
fn where_not_com_parenteses() {
    // NOT (age < 18 OR active = FALSE)
    let c = not(Condition::Parens(Box::new(or(
        cmp(id("age"), Operator::Less, Value::Int(18)),
        cmp(id("active"), Operator::Equal, Value::Boolean(false)),
    ))));
    assert_eq!(names_where(c), ["ana", "carla", "diego", "elisa"]);
}

#[test]
fn where_in() {
    let c = Condition::ExprIn(ExprIn { value: id("type"), values: vec![string("admin"), string("viewer")] });
    assert_eq!(names_where(c), ["ana", "carla", "diego"]);
}

#[test]
fn where_in_numerico() {
    let c = Condition::ExprIn(ExprIn { value: id("age"), values: vec![Value::Int(17), Value::Int(42), Value::Int(99)] });
    assert_eq!(names_where(c), ["bruno", "diego"]);
}

#[test]
fn exemplo_do_readme() {
    // type IN ('admin', 'editor') AND active = TRUE AND credit >= 150.50
    let c = and(
        and(
            Condition::ExprIn(ExprIn { value: id("type"), values: vec![string("admin"), string("editor")] }),
            cmp(id("active"), Operator::Equal, Value::Boolean(true)),
        ),
        cmp(id("credit"), Operator::GreaterEqual, Value::Float(150.50)),
    );
    assert_eq!(names_where(c), ["ana", "elisa"]);
}

// ----- NULL (célula vazia) -----------------------------------------------

#[test]
fn null_nao_satisfaz_comparacao() {
    // diego tem credit vazio: não entra em credit > 0 ...
    assert!(!names_where(cmp(id("credit"), Operator::Greater, Value::Int(0))).contains(&"diego".to_string()));
}

#[test]
fn null_nao_satisfaz_nem_a_negacao() {
    // ... e também não entra em NOT (credit > 0): o resultado é "desconhecido".
    let result = names_where(not(cmp(id("credit"), Operator::Greater, Value::Int(0))));
    assert!(result.is_empty(), "{result:?}");
}

#[test]
fn null_com_or_verdadeiro_entra() {
    // desconhecido OR verdadeiro = verdadeiro
    let c = or(cmp(id("credit"), Operator::Greater, Value::Int(0)), cmp(id("name"), Operator::Equal, string("diego")));
    assert!(names_where(c).contains(&"diego".to_string()));
}

#[test]
fn null_com_and_falso_e_falso() {
    // desconhecido AND falso = falso -> NOT(...) = verdadeiro
    let c = not(and(cmp(id("credit"), Operator::Greater, Value::Int(0)), cmp(id("age"), Operator::Less, Value::Int(0))));
    assert_eq!(names_where(c).len(), 5);
}