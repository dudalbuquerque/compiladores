// As ASTs são montadas à mão (sem passar pelo parser) para isolar o type checker
// Tabela usada: test_data/users.csv
// name: String | age: Int | type: String | credit: Float | active: Boolean

use mini_sql::ast::*;
use mini_sql::csv_loader::load_csv;
use mini_sql::table::Database;
use mini_sql::type_checker::check_statement;

// auxiliares

fn db() -> Database {
    let mut db = Database::new();
    db.insert("users".to_string(), load_csv("test_data/users.csv").expect("users.csv"));
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

fn query(table: &str, select: SelectList, condition: Option<Condition>) -> Statement {
    Statement::Query(Query { table: table.to_string(), select, condition })
}

fn where_(condition: Condition) -> Statement {
    query("users", SelectList::All, Some(condition))
}

fn check(statement: Statement) -> Result<(), String> {
    check_statement(&statement, &db())
}

fn assert_ok(statement: Statement) {
    if let Err(e) = check(statement) {
        panic!("esperava programa bem tipado, mas deu erro: {e}");
    }
}

fn assert_err(statement: Statement, fragment: &str) {
    match check(statement) {
        Ok(()) => panic!("esperava erro contendo '{fragment}', mas passou"),
        Err(e) => assert!(e.contains(fragment), "mensagem '{e}' não contém '{fragment}'"),
    }
}

// ----- tabelas e colunas -------------------------------------------------

#[test]
fn query_valida_com_todas_as_colunas() {
    assert_ok(query("users", SelectList::All, None));
}

#[test]
fn query_valida_com_colunas() {
    assert_ok(query("users", SelectList::Columns(vec!["name".into(), "credit".into()]), None));
}

#[test]
fn tabela_inexistente() {
    assert_err(query("clientes", SelectList::All, None), "Tabela 'clientes' não encontrada");
}

#[test]
fn coluna_inexistente_no_select() {
    assert_err(
        query("users", SelectList::Columns(vec!["name".into(), "email".into()]), None),
        "Coluna 'email' não existe",
    );
}

#[test]
fn coluna_inexistente_no_where() {
    assert_err(where_(cmp(id("salary"), Operator::Greater, Value::Int(10))), "Coluna 'salary'");
}

#[test]
fn coluna_inexistente_dentro_de_not_and_or() {
    let condition = Condition::Not(Box::new(Condition::Or(
        Box::new(cmp(id("age"), Operator::Greater, Value::Int(18))),
        Box::new(cmp(id("cidade"), Operator::Equal, string("recife"))),
    )));
    assert_err(where_(condition), "Coluna 'cidade'");
}

#[test]
fn describe_tabela_existente() {
    assert_ok(Statement::Describe(Describe { table: "users".into() }));
}

#[test]
fn describe_tabela_inexistente() {
    assert_err(Statement::Describe(Describe { table: "nada".into() }), "Tabela 'nada'");
}

// ----- comparações -------------------------------------------------------

#[test]
fn comparacao_mesmo_tipo() {
    assert_ok(where_(cmp(id("age"), Operator::GreaterEqual, Value::Int(18))));
    assert_ok(where_(cmp(id("name"), Operator::Equal, string("ana"))));
    assert_ok(where_(cmp(id("active"), Operator::Equal, Value::Boolean(true))));
    assert_ok(where_(cmp(id("credit"), Operator::Less, Value::Float(100.0))));
}

//como estabelecido no table.rs, int e float são compatíveis
#[test]
fn int_e_float_sao_compativeis() {
    // credit é Float, 150 é Int
    assert_ok(where_(cmp(id("credit"), Operator::GreaterEqual, Value::Int(150))));
    // age é Int, 17.5 é Float
    assert_ok(where_(cmp(id("age"), Operator::Less, Value::Float(17.5))));
}

#[test]
fn comparacao_entre_duas_colunas() {
    assert_ok(where_(cmp(id("age"), Operator::Less, id("credit"))));
    assert_err(where_(cmp(id("age"), Operator::Equal, id("name"))), "Tipos incompatíveis");
}

#[test]
fn literal_a_esquerda() {
    assert_ok(where_(cmp(Value::Int(18), Operator::Less, id("age"))));
}

#[test]
fn int_com_string_e_erro() {
    assert_err(where_(cmp(id("age"), Operator::Equal, string("vinte"))), "Tipos incompatíveis");
}

#[test]
fn boolean_com_int_e_erro() {
    assert_err(where_(cmp(id("active"), Operator::Equal, Value::Int(1))), "Tipos incompatíveis");
}

#[test]
fn literais_incompativeis() {
    assert_err(where_(cmp(Value::Int(1), Operator::Equal, string("1"))), "Tipos incompatíveis");
}

#[test]
fn ordem_em_string_e_permitida() {
    assert_ok(where_(cmp(id("name"), Operator::Less, string("c"))));
}

#[test]
fn ordem_em_boolean_e_erro() {
    assert_err(where_(cmp(id("active"), Operator::Greater, Value::Boolean(false))), "não se aplica ao tipo Boolean");
}

#[test]
fn mensagem_de_erro_mostra_a_expressao() {
    let err = check(where_(cmp(id("age"), Operator::Equal, string("vinte")))).unwrap_err();
    assert!(err.contains("age = 'vinte'"), "{err}");
    assert!(err.contains("Int"), "{err}");
    assert!(err.contains("String"), "{err}");
}

// ----- IN ----------------------------------------------------------------

#[test]
fn in_com_tipos_corretos() {
    let condition = Condition::ExprIn(ExprIn { value: id("type"), values: vec![string("admin"), string("editor")] });
    assert_ok(where_(condition));
}

#[test]
fn in_numerico_misturando_int_e_float() {
    let condition = Condition::ExprIn(ExprIn { value: id("credit"), values: vec![Value::Int(100), Value::Float(150.5)] });
    assert_ok(where_(condition));
}

#[test]
fn in_com_item_de_tipo_errado() {
    let condition = Condition::ExprIn(ExprIn { value: id("type"), values: vec![string("admin"), Value::Int(3)] });
    assert_err(where_(condition), "Tipos incompatíveis no IN");
}

// ----- LOAD --------------------------------------------------------------

#[test]
fn load_csv_valido() {
    assert_ok(Statement::Load(Load { file: "test_data/alunos.csv".into() }));
}

#[test]
fn load_de_arquivo_que_nao_e_csv() {
    assert_err(Statement::Load(Load { file: "dados.txt".into() }), "espera um arquivo .csv");
}

#[test]
fn load_com_nome_que_nao_vira_identificador() {
    assert_err(Statement::Load(Load { file: "Alunos.csv".into() }), "não é um identificador válido");
    assert_err(Statement::Load(Load { file: "alunos-2024.csv".into() }), "não é um identificador válido");
}