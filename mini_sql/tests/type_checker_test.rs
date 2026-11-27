// As ASTs são montadas à mão (sem passar pelo parser) para isolar o type checker
// Tabelas usadas:
//   users.csv:    name: String | age: Int | type: String | credit: Float | active: Boolean
//   usuarios.csv: id: Int | nome: String | idade: Int
//   pedidos.csv:  id: Int | usuario_id: Int | produto: String | valor: Float

use mini_sql::ast::*;
use mini_sql::csv_loader::load_csv;
use mini_sql::table::Database;
use mini_sql::type_checker::check_statement;

// auxiliares

fn db() -> Database {
    let mut db = Database::new();
    db.insert("users".to_string(), load_csv("test_data/users.csv").expect("users.csv"));
    // usadas nos testes de JOIN e nomes qualificados (as duas têm uma coluna `id`)
    db.insert("usuarios".to_string(), load_csv("test_data/usuarios.csv").expect("usuarios.csv"));
    db.insert("pedidos".to_string(), load_csv("test_data/pedidos.csv").expect("pedidos.csv"));
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
    Statement::Query(Query {
        ctes: vec![],
        body: QueryBody { table: table.to_string(), joins: vec![], select, condition },
    })
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

// ----- auxiliares para CTEs e JOINs --------------------------------------

fn columns(names: &[&str]) -> SelectList {
    SelectList::Columns(names.iter().map(|n| n.to_string()).collect())
}

/// `FROM table [joins] SELECT select [WHERE condition]`
fn body(table: &str, joins: Vec<Join>, select: SelectList, condition: Option<Condition>) -> QueryBody {
    QueryBody { table: table.to_string(), joins, select, condition }
}

/// `INNER JOIN table ON left = right`
fn join(table: &str, left: &str, right: &str) -> Join {
    Join {
        join_type: JoinType::Inner,
        table: table.to_string(),
        condition: Expr { left: id(left), op: Operator::Equal, right: id(right) },
    }
}

/// `name [(c1, c2)] AS (body)`
fn cte(name: &str, cols: Option<&[&str]>, body: QueryBody) -> Cte {
    Cte {
        name: name.to_string(),
        columns: cols.map(|c| c.iter().map(|n| n.to_string()).collect()),
        body,
    }
}

/// `WITH ctes main;`
fn with(ctes: Vec<Cte>, main: QueryBody) -> Statement {
    Statement::Query(Query { ctes, body: main })
}

fn select(main: QueryBody) -> Statement {
    with(vec![], main)
}

/// `FROM users SELECT name, age WHERE age >= 18`
fn adultos_body() -> QueryBody {
    body("users", vec![], columns(&["name", "age"]), Some(cmp(id("age"), Operator::GreaterEqual, Value::Int(18))))
}

// ----- CTEs: visibilidade e escopo ---------------------------------------

#[test]
fn cte_e_visivel_no_corpo() {
    // WITH adultos AS (FROM users SELECT name, age WHERE age >= 18) FROM adultos SELECT name;
    assert_ok(with(vec![cte("adultos", None, adultos_body())], body("adultos", vec![], columns(&["name"]), None)));
}

#[test]
fn cte_so_tem_as_colunas_do_proprio_select() {
    // `adultos` só tem name e age: credit existe em users, mas não na CTE
    assert_err(
        with(vec![cte("adultos", None, adultos_body())], body("adultos", vec![], columns(&["credit"]), None)),
        "Coluna 'credit' não existe",
    );
}

#[test]
fn cte_enxerga_as_ctes_anteriores() {
    // WITH a AS (FROM users SELECT name), b AS (FROM a SELECT name) FROM b SELECT name;
    assert_ok(with(
        vec![
            cte("a", None, body("users", vec![], columns(&["name"]), None)),
            cte("b", None, body("a", vec![], columns(&["name"]), None)),
        ],
        body("b", vec![], columns(&["name"]), None),
    ));
}

#[test]
fn cte_nao_enxerga_as_ctes_seguintes() {
    // WITH b AS (FROM a ...), a AS (...) -> `a` ainda não existe quando `b` é verificada
    assert_err(
        with(
            vec![
                cte("b", None, body("a", vec![], columns(&["name"]), None)),
                cte("a", None, body("users", vec![], columns(&["name"]), None)),
            ],
            body("b", vec![], columns(&["name"]), None),
        ),
        "Na CTE 'b': Tabela 'a' não encontrada",
    );
}

#[test]
fn cte_com_nome_repetido_no_mesmo_with() {
    assert_err(
        with(
            vec![
                cte("a", None, body("users", vec![], columns(&["name"]), None)),
                cte("a", None, body("users", vec![], columns(&["age"]), None)),
            ],
            body("a", vec![], columns(&["name"]), None),
        ),
        "CTE 'a' definida mais de uma vez",
    );
}

#[test]
fn cte_esconde_tabela_carregada_com_o_mesmo_nome() {
    // WITH users AS (FROM usuarios SELECT nome) FROM users SELECT nome;
    // Dentro desta query, `users` é a CTE, não a tabela carregada.
    let shadow = || cte("users", None, body("usuarios", vec![], columns(&["nome"]), None));

    assert_ok(with(vec![shadow()], body("users", vec![], columns(&["nome"]), None)));
    assert_err(with(vec![shadow()], body("users", vec![], columns(&["name"]), None)), "Coluna 'name' não existe");
}

#[test]
fn cte_que_cita_o_proprio_nome_le_a_tabela_carregada() {
    // WITH users AS (FROM users SELECT name) ... : o `users` de dentro ainda é a
    // tabela carregada, porque a CTE só passa a existir depois de verificada.
    assert_ok(with(
        vec![cte("users", None, body("users", vec![], columns(&["name"]), None))],
        body("users", vec![], columns(&["name"]), None),
    ));
}

#[test]
fn cte_nao_existe_fora_da_sua_query() {
    // A checagem de uma query com WITH não altera o Database...
    let database = db();
    let statement = with(vec![cte("adultos", None, adultos_body())], body("adultos", vec![], SelectList::All, None));
    check_statement(&statement, &database).unwrap();

    // ...então, na query seguinte, `adultos` não existe.
    let err = check_statement(&select(body("adultos", vec![], SelectList::All, None)), &database).unwrap_err();
    assert!(err.contains("Tabela 'adultos' não encontrada"), "{err}");
}

#[test]
fn erro_dentro_da_cte_diz_qual_cte() {
    let bad = body("users", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::Equal, string("vinte"))));
    assert_err(with(vec![cte("a", None, bad)], body("a", vec![], columns(&["name"]), None)), "Na CTE 'a': Tipos incompatíveis");
}

#[test]
fn erro_no_corpo_continua_sendo_detectado() {
    // WITH adultos AS (...) FROM adultos SELECT name WHERE age = 'vinte';
    assert_err(
        with(
            vec![cte("adultos", None, adultos_body())],
            body("adultos", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::Equal, string("vinte")))),
        ),
        "Tipos incompatíveis",
    );
}

// ----- CTEs: lista de colunas --------------------------------------------

#[test]
fn lista_de_colunas_renomeia() {
    // WITH adultos (nome, anos) AS (FROM users SELECT name, age ...) FROM adultos SELECT nome WHERE anos > 20;
    let renamed = || cte("adultos", Some(&["nome", "anos"]), adultos_body());

    assert_ok(with(
        vec![renamed()],
        body("adultos", vec![], columns(&["nome"]), Some(cmp(id("anos"), Operator::Greater, Value::Int(20)))),
    ));
    // o nome antigo deixa de existir
    assert_err(with(vec![renamed()], body("adultos", vec![], columns(&["name"]), None)), "Coluna 'name' não existe");
}

#[test]
fn lista_de_colunas_mantem_os_tipos() {
    // `anos` veio de `age`, então continua Int
    assert_err(
        with(
            vec![cte("adultos", Some(&["nome", "anos"]), adultos_body())],
            body("adultos", vec![], columns(&["nome"]), Some(cmp(id("anos"), Operator::Equal, string("vinte")))),
        ),
        "a coluna 'anos' é Int",
    );
}

#[test]
fn lista_de_colunas_com_quantidade_errada() {
    assert_err(
        with(
            vec![cte("a", Some(&["x", "y"]), body("users", vec![], columns(&["name"]), None))],
            body("a", vec![], columns(&["x"]), None),
        ),
        "A CTE 'a' declara 2 coluna(s) (x, y), mas o SELECT dela produz 1",
    );
}

#[test]
fn cte_com_coluna_repetida_e_erro() {
    // `SELECT *` de usuarios JOIN pedidos produz `id` duas vezes
    let star = body("usuarios", vec![join("pedidos", "usuarios.id", "usuario_id")], SelectList::All, None);
    assert_err(
        with(vec![cte("c", None, star)], body("c", vec![], columns(&["nome"]), None)),
        "A CTE 'c' produz a coluna 'id' mais de uma vez",
    );
}

#[test]
fn lista_de_colunas_resolve_a_repeticao() {
    let star = body("usuarios", vec![join("pedidos", "usuarios.id", "usuario_id")], SelectList::All, None);
    let names = ["uid", "nome", "idade", "pid", "usuario_id", "produto", "valor"];
    assert_ok(with(vec![cte("c", Some(&names), star)], body("c", vec![], columns(&["uid", "pid"]), None)));
}

#[test]
fn lista_de_colunas_com_nome_repetido_e_erro() {
    // WITH a (x, x) AS (FROM users SELECT name, age)
    assert_err(
        with(
            vec![cte("a", Some(&["x", "x"]), body("users", vec![], columns(&["name", "age"]), None))],
            body("a", vec![], columns(&["x"]), None),
        ),
        "produz a coluna 'x' mais de uma vez",
    );
}

// ----- CTEs com JOIN -----------------------------------------------------

#[test]
fn cte_com_join_no_corpo() {
    // WITH c AS (FROM usuarios JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto) FROM c SELECT produto;
    let joined = body("usuarios", vec![join("pedidos", "usuarios.id", "usuario_id")], columns(&["nome", "produto"]), None);
    assert_ok(with(vec![cte("c", None, joined)], body("c", vec![], columns(&["produto"]), None)));
}

#[test]
fn join_com_uma_cte() {
    // WITH a AS (FROM users SELECT name) FROM a JOIN users ON a.name = users.name SELECT a.name, age;
    assert_ok(with(
        vec![cte("a", None, body("users", vec![], columns(&["name"]), None))],
        body("a", vec![join("users", "a.name", "users.name")], columns(&["a.name", "age"]), None),
    ));
}

// ----- nomes qualificados e ambiguidade ----------------------------------

#[test]
fn coluna_qualificada_numa_query_simples() {
    // FROM users SELECT users.name WHERE users.age >= 30;
    assert_ok(select(body(
        "users",
        vec![],
        columns(&["users.name"]),
        Some(cmp(id("users.age"), Operator::GreaterEqual, Value::Int(30))),
    )));
}

#[test]
fn coluna_ambigua_no_on() {
    // FROM usuarios JOIN pedidos ON id = usuario_id: `id` existe nas duas tabelas
    assert_err(
        select(body("usuarios", vec![join("pedidos", "id", "usuario_id")], columns(&["nome"]), None)),
        "Coluna 'id' é ambígua: existe em usuarios e pedidos; escreva usuarios.id ou pedidos.id",
    );
}

#[test]
fn coluna_ambigua_no_select() {
    assert_err(
        select(body("usuarios", vec![join("pedidos", "usuarios.id", "usuario_id")], columns(&["id"]), None)),
        "Coluna 'id' é ambígua",
    );
}

#[test]
fn coluna_ambigua_no_where() {
    assert_err(
        select(body(
            "usuarios",
            vec![join("pedidos", "usuarios.id", "usuario_id")],
            columns(&["nome"]),
            Some(cmp(id("id"), Operator::Greater, Value::Int(1))),
        )),
        "Coluna 'id' é ambígua",
    );
}

#[test]
fn mesma_coluna_dos_dois_lados_e_ambigua() {
    // O bug original: `ON name = name` comparava a coluna com ela mesma
    // (sempre verdadeiro) e devolvia um produto cartesiano.
    assert_err(
        with(
            vec![cte("a", None, body("users", vec![], columns(&["name"]), None))],
            body("a", vec![join("users", "name", "name")], columns(&["age"]), None),
        ),
        "Coluna 'name' é ambígua: existe em a e users",
    );
}

#[test]
fn prefixo_resolve_a_ambiguidade() {
    assert_ok(select(body(
        "usuarios",
        vec![join("pedidos", "usuarios.id", "pedidos.usuario_id")],
        columns(&["usuarios.id", "pedidos.id", "nome"]),
        Some(cmp(id("pedidos.valor"), Operator::Greater, Value::Int(100))),
    )));
}

#[test]
fn coluna_que_so_existe_numa_tabela_dispensa_prefixo() {
    // `usuario_id` só existe em pedidos: não é ambígua
    assert_ok(select(body("usuarios", vec![join("pedidos", "usuarios.id", "usuario_id")], columns(&["nome", "produto"]), None)));
}

#[test]
fn prefixo_com_tabela_que_nao_esta_na_query() {
    assert_err(
        select(body("usuarios", vec![join("pedidos", "clientes.id", "usuario_id")], columns(&["nome"]), None)),
        "Tabela 'clientes' não faz parte desta consulta",
    );
}

#[test]
fn coluna_qualificada_que_nao_existe() {
    assert_err(
        select(body("usuarios", vec![join("pedidos", "usuarios.email", "usuario_id")], columns(&["nome"]), None)),
        "Coluna 'usuarios.email' não existe",
    );
}

#[test]
fn mesma_tabela_duas_vezes_e_ambigua_mesmo_com_prefixo() {
    // Sem apelidos (AS), `users.name` não distingue as duas cópias de users
    assert_err(
        select(body("users", vec![join("users", "users.name", "users.name")], columns(&["age"]), None)),
        "a tabela 'users' aparece mais de uma vez",
    );
}
