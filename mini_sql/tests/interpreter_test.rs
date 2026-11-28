// Testes do interpretador.

// test_data/users.csv:

use mini_sql::ast::*;
use mini_sql::interpreter::{execute_describe, execute_load, execute_query, execute_statement, execute_statement_traced};
use mini_sql::table::{DataType, Database};

// ----- auxiliares --------------------------------------------------------

/// Query sem WITH: só o corpo.
fn simple(body: QueryBody) -> Query {
    Query { ctes: vec![], body }
}

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
    let query = simple(QueryBody {
        table: "users".into(),
        joins: vec![],
        select: columns(&["name"]),
        condition: Some(condition),
    });
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

    assert_eq!(msg, "Tabela 'alunos' carregada.");
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
    let query = simple(QueryBody { table: "users".into(), joins: vec![], select: SelectList::All, condition: None });
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(result.columns.len(), 5);
    assert_eq!(result.rows.len(), 5);
}

#[test]
fn select_projeta_na_ordem_pedida() {
    let query = simple(QueryBody {
        table: "users".into(),
        joins: vec![],
        select: columns(&["credit", "name"]),
        condition: None,
    });
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(result.columns[0].name, "credit");
    assert_eq!(result.rows[0], vec!["200.75", "ana"]);
}

#[test]
fn saida_formatada_em_tabela() {
    let query = simple(QueryBody {
        table: "users".into(),
        joins: vec![],
        select: columns(&["name", "age"]),
        condition: Some(cmp(id("age"), Operator::Less, Value::Int(20))),
    });
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

// ----- Funções de agregação ------------------------------------------------

fn agg(func: AggregateFunc, column: Option<&str>, alias: Option<&str>) -> SelectItem {
    SelectItem::Aggregate {
        func,
        arg: match column {
            Some(name) => AggArg::Column(name.to_string()),
            None => AggArg::Star,
        },
        alias: alias.map(|a| a.to_string()),
    }
}

fn col_item(name: &str, alias: Option<&str>) -> SelectItem {
    SelectItem::Column { name: name.to_string(), alias: alias.map(|a| a.to_string()) }
}

/// `FROM <table> SELECT <items> [WHERE ...]` executado direto no interpretador.
fn run_items(table: &str, items: Vec<SelectItem>, condition: Option<Condition>) -> Result<mini_sql::table::Table, String> {
    let query = simple(body(table, vec![], SelectList::Items(items), condition));
    execute_query(&query, &db_all())
}

#[test]
fn count_star_conta_as_linhas() {
    let result = run_items("users", vec![agg(AggregateFunc::Count, None, None)], None).unwrap();

    assert_eq!(result.rows, vec![vec!["5"]]);
    assert_eq!(column_names(&result), vec!["COUNT(*)"]);
    assert_eq!(result.columns[0].data_type, DataType::Int);
}

#[test]
fn alias_vira_o_nome_da_coluna_do_resultado() {
    let result = run_items("users", vec![agg(AggregateFunc::Count, None, Some("total"))], None).unwrap();

    assert_eq!(column_names(&result), vec!["total"]);
}

#[test]
fn count_star_respeita_o_where() {
    let result = run_items(
        "users",
        vec![agg(AggregateFunc::Count, None, None)],
        Some(cmp(id("age"), Operator::Less, Value::Int(18))),
    )
    .unwrap();

    assert_eq!(result.rows, vec![vec!["1"]]);
}

#[test]
fn count_star_sem_linhas_da_zero() {
    let result = run_items(
        "users",
        vec![agg(AggregateFunc::Count, None, None)],
        Some(cmp(id("age"), Operator::Greater, Value::Int(1000))),
    )
    .unwrap();

    assert_eq!(result.rows, vec![vec!["0"]]);
}

#[test]
fn count_coluna_ignora_null_mas_count_star_nao() {
    // diego tem credit vazio
    let result = run_items(
        "users",
        vec![agg(AggregateFunc::Count, Some("credit"), None), agg(AggregateFunc::Count, None, None)],
        None,
    )
    .unwrap();

    assert_eq!(result.rows, vec![vec!["4", "5"]]);
}

#[test]
fn count_de_coluna_de_texto_funciona() {
    let result = run_items("users", vec![agg(AggregateFunc::Count, Some("name"), None)], None).unwrap();

    assert_eq!(result.rows, vec![vec!["5"]]);
}

#[test]
fn sum_avg_min_max_sobre_float() {
    // pedidos.valor: 150, 80, 900, 200
    let result = run_items(
        "pedidos",
        vec![
            agg(AggregateFunc::Sum, Some("valor"), None),
            agg(AggregateFunc::Avg, Some("valor"), None),
            agg(AggregateFunc::Min, Some("valor"), None),
            agg(AggregateFunc::Max, Some("valor"), None),
        ],
        None,
    )
    .unwrap();

    assert_eq!(result.rows, vec![vec!["1330", "332.5", "80", "900"]]);
    assert_eq!(column_names(&result), vec!["SUM(valor)", "AVG(valor)", "MIN(valor)", "MAX(valor)"]);
}

#[test]
fn tipos_do_resultado_das_agregacoes() {
    // SUM/MIN/MAX mantêm o tipo da coluna (Int); AVG é sempre Float
    let result = run_items(
        "usuarios",
        vec![
            agg(AggregateFunc::Sum, Some("idade"), None),
            agg(AggregateFunc::Max, Some("idade"), None),
            agg(AggregateFunc::Avg, Some("idade"), None),
        ],
        None,
    )
    .unwrap();

    assert_eq!(result.rows[0][0], "77");
    assert_eq!(result.rows[0][1], "30");
    let types: Vec<DataType> = result.columns.iter().map(|c| c.data_type.clone()).collect();
    assert_eq!(types, vec![DataType::Int, DataType::Int, DataType::Float]);
}

#[test]
fn agregacao_sem_valores_da_null_e_count_da_zero() {
    // nenhum usuário tem idade > 1000: SUM/AVG/MIN/MAX viram NULL (vazio), COUNT vira 0
    let result = run_items(
        "usuarios",
        vec![
            agg(AggregateFunc::Sum, Some("idade"), None),
            agg(AggregateFunc::Avg, Some("idade"), None),
            agg(AggregateFunc::Min, Some("idade"), None),
            agg(AggregateFunc::Count, Some("idade"), None),
        ],
        Some(cmp(id("idade"), Operator::Greater, Value::Int(1000))),
    )
    .unwrap();

    assert_eq!(result.rows, vec![vec!["", "", "", "0"]]);
}

#[test]
fn agregacao_com_where_e_prefixo_em_join() {
    // FROM usuarios JOIN pedidos ON usuarios.id = usuario_id SELECT SUM(pedidos.valor);
    // Ana: 150 + 80, Bruno: 900
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Inner, "pedidos", "usuarios.id", "usuario_id")],
        SelectList::Items(vec![agg(AggregateFunc::Sum, Some("pedidos.valor"), Some("soma"))]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(result.rows, vec![vec!["1130"]]);
    assert_eq!(column_names(&result), vec!["soma"]);
}

#[test]
fn count_ignora_null_do_left_join() {
    // LEFT JOIN: Carlos entra com valor NULL. COUNT(*) conta 4 linhas, COUNT(valor) só 3.
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Left, "pedidos", "usuarios.id", "usuario_id")],
        SelectList::Items(vec![agg(AggregateFunc::Count, None, None), agg(AggregateFunc::Count, Some("valor"), None)]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(result.rows, vec![vec!["4", "3"]]);
}

#[test]
fn alias_em_coluna_comum() {
    let result = run_items("usuarios", vec![col_item("nome", Some("n")), col_item("idade", None)], None).unwrap();

    assert_eq!(column_names(&result), vec!["n", "idade"]);
    assert_eq!(result.rows[0], vec!["Ana", "25"]);
}

#[test]
fn cte_com_agregacao_e_lida_pelo_alias() {
    // WITH t AS (FROM users SELECT COUNT(*) AS total) FROM t SELECT total WHERE total > 3;
    let query = with(
        vec![cte(
            "t",
            None,
            body("users", vec![], SelectList::Items(vec![agg(AggregateFunc::Count, None, Some("total"))]), None),
        )],
        body("t", vec![], columns(&["total"]), Some(cmp(id("total"), Operator::Greater, Value::Int(3)))),
    );
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(result.rows, vec![vec!["5"]]);
}

// ----- agregações: erros (proteção do interpretador) -----------------------
// Em uso normal o type checker recusa essas queries antes de executar;
// aqui o interpretador é chamado direto, para provar que ele também não aceita.

#[test]
fn misturar_coluna_e_agregacao_e_erro() {
    let err = run_items(
        "users",
        vec![col_item("name", None), agg(AggregateFunc::Count, None, None)],
        None,
    )
    .unwrap_err();

    assert!(err.contains("GROUP BY"), "{err}");
}

#[test]
fn sum_de_coluna_de_texto_e_erro() {
    let err = run_items("users", vec![agg(AggregateFunc::Sum, Some("name"), None)], None).unwrap_err();

    assert!(err.contains("exige coluna numérica"), "{err}");
}

#[test]
fn star_so_vale_para_count() {
    let err = run_items("users", vec![agg(AggregateFunc::Sum, None, None)], None).unwrap_err();

    assert!(err.contains("só COUNT aceita *"), "{err}");
}

#[test]
fn agregacao_de_coluna_inexistente_e_erro() {
    assert!(run_items("users", vec![agg(AggregateFunc::Max, Some("nao_existe"), None)], None).is_err());
}

// ----- auxiliares para CTEs e JOINs --------------------------------------
//
// usuarios.csv: (1, Ana, 25) (2, Bruno, 30) (3, Carlos, 22)
// pedidos.csv:  (101, 1, Teclado, 150.0) (102, 1, Mouse, 80.0)
//               (103, 2, Monitor, 900.0) (104, 4, Headset, 200.0)
//   Carlos não tem pedido; o pedido 104 é de um usuário que não existe.

/// users + usuarios + pedidos
fn db_all() -> Database {
    let mut db = db();
    for file in ["test_data/usuarios.csv", "test_data/pedidos.csv"] {
        execute_load(&Load { file: file.into() }, &mut db).expect("LOAD");
    }
    db
}

fn body(table: &str, joins: Vec<Join>, select: SelectList, condition: Option<Condition>) -> QueryBody {
    QueryBody { table: table.to_string(), joins, select, condition }
}

fn columns(names: &[&str]) -> SelectList {
    SelectList::Items(
        names
            .iter()
            .map(|n| SelectItem::Column { name: n.to_string(), alias: None })
            .collect(),
    )
}

/// `[INNER|LEFT] JOIN table ON left = right`
fn join(join_type: JoinType, table: &str, left: &str, right: &str) -> Join {
    Join {
        join_type,
        table: table.to_string(),
        condition: Expr { left: id(left), op: Operator::Equal, right: id(right) },
    }
}

fn cte(name: &str, cols: Option<&[&str]>, body: QueryBody) -> Cte {
    Cte {
        name: name.to_string(),
        columns: cols.map(|c| c.iter().map(|n| n.to_string()).collect()),
        body,
    }
}

fn with(ctes: Vec<Cte>, main: QueryBody) -> Query {
    Query { ctes, body: main }
}

/// Primeira coluna de cada linha do resultado.
fn first_column(table: &mini_sql::table::Table) -> Vec<String> {
    table.rows.iter().map(|row| row[0].clone()).collect()
}

fn column_names(table: &mini_sql::table::Table) -> Vec<String> {
    table.columns.iter().map(|c| c.name.clone()).collect()
}

/// `FROM users SELECT name, age WHERE age >= 18` (todos menos bruno, que tem 17)
fn adultos_body() -> QueryBody {
    body("users", vec![], columns(&["name", "age"]), Some(cmp(id("age"), Operator::GreaterEqual, Value::Int(18))))
}

// ----- CTEs --------------------------------------------------------------

#[test]
fn cte_vira_tabela_temporaria_lida_pelo_corpo() {
    // WITH adultos AS (...) FROM adultos SELECT name WHERE age > 30;
    let query = with(
        vec![cte("adultos", None, adultos_body())],
        body("adultos", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::Greater, Value::Int(30)))),
    );
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(first_column(&result), vec!["carla", "diego"]);
}

#[test]
fn cte_encadeada_usa_o_resultado_da_anterior() {
    // WITH adultos AS (...), velhos AS (FROM adultos SELECT name WHERE age >= 35) FROM velhos SELECT name;
    let query = with(
        vec![
            cte("adultos", None, adultos_body()),
            cte("velhos", None, body("adultos", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::GreaterEqual, Value::Int(35))))),
        ],
        body("velhos", vec![], columns(&["name"]), None),
    );
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(first_column(&result), vec!["carla", "diego"]);
}

#[test]
fn lista_de_colunas_renomeia_e_mantem_os_tipos() {
    // WITH adultos (nome, anos) AS (FROM users SELECT name, age ...) FROM adultos SELECT *;
    let query = with(vec![cte("adultos", Some(&["nome", "anos"]), adultos_body())], body("adultos", vec![], SelectList::All, None));
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(column_names(&result), vec!["nome", "anos"]);
    assert_eq!(result.columns[1].data_type, DataType::Int);
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn coluna_renomeada_funciona_no_where() {
    let query = with(
        vec![cte("adultos", Some(&["nome", "anos"]), adultos_body())],
        body("adultos", vec![], columns(&["nome"]), Some(cmp(id("anos"), Operator::Less, Value::Int(30)))),
    );
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(first_column(&result), vec!["ana", "elisa"]);
}

#[test]
fn cte_nao_altera_o_database() {
    // A tabela temporária vive só durante a query: o Database sai igual.
    let database = db();
    let before: Vec<String> = database.keys().cloned().collect();

    let query = with(vec![cte("adultos", None, adultos_body())], body("adultos", vec![], SelectList::All, None));
    execute_query(&query, &database).unwrap();

    let after: Vec<String> = database.keys().cloned().collect();
    assert_eq!(before, after);
    assert!(!database.contains_key("adultos"));
}

#[test]
fn cte_esconde_tabela_carregada_com_o_mesmo_nome() {
    // WITH users AS (FROM usuarios SELECT nome) FROM users SELECT nome;
    let query = with(
        vec![cte("users", None, body("usuarios", vec![], columns(&["nome"]), None))],
        body("users", vec![], columns(&["nome"]), None),
    );
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(first_column(&result), vec!["Ana", "Bruno", "Carlos"]);
}

#[test]
fn cte_que_cita_o_proprio_nome_le_a_tabela_carregada() {
    // WITH users AS (FROM users SELECT name WHERE age < 18) FROM users SELECT name;
    let query = with(
        vec![cte("users", None, body("users", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::Less, Value::Int(18)))))],
        body("users", vec![], columns(&["name"]), None),
    );
    let result = execute_query(&query, &db()).unwrap();

    assert_eq!(first_column(&result), vec!["bruno"]);
}

#[test]
fn cte_com_join_e_join_com_cte() {
    // WITH compras AS (FROM usuarios JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto, valor)
    // FROM compras SELECT nome, produto WHERE valor >= 150;
    let compras = body(
        "usuarios",
        vec![join(JoinType::Inner, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto", "valor"]),
        None,
    );
    let query = with(
        vec![cte("compras", None, compras)],
        body("compras", vec![], columns(&["nome", "produto"]), Some(cmp(id("valor"), Operator::GreaterEqual, Value::Int(150)))),
    );
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(result.rows, vec![vec!["Ana", "Teclado"], vec!["Bruno", "Monitor"]]);
}

// ----- trace (tabelas temporárias visíveis no terminal) ------------------

#[test]
fn trace_mostra_criacao_e_descarte_das_ctes() {
    let statement = Statement::Query(with(
        vec![
            cte("adultos", None, adultos_body()),
            cte("velhos", None, body("adultos", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::GreaterEqual, Value::Int(35))))),
        ],
        body("velhos", vec![], columns(&["name"]), None),
    ));
    let lines = execute_statement_traced(&statement, &mut db(), true).unwrap();

    let position = |text: &str| lines.iter().position(|l| l == text).unwrap_or_else(|| panic!("falta '{text}' em {lines:#?}"));
    let created_adultos = position("[CTE] tabela temporária 'adultos' criada:");
    let created_velhos = position("[CTE] tabela temporária 'velhos' criada:");
    let discarded = position("[CTE] tabelas temporárias descartadas: adultos, velhos");

    // ordem: cria adultos, cria velhos, descarta as duas, e só então o resultado
    assert!(created_adultos < created_velhos && created_velhos < discarded);
    assert_eq!(lines[created_adultos + 1], "name  | age"); // conteúdo da tabela temporária
    assert_eq!(lines.last().unwrap(), "(2 linhas)"); // resultado final no fim
}

#[test]
fn sem_trace_a_saida_e_so_o_resultado() {
    let statement = Statement::Query(with(vec![cte("adultos", None, adultos_body())], body("adultos", vec![], columns(&["name"]), None)));

    let traced_off = execute_statement_traced(&statement, &mut db(), false).unwrap();
    let plain = execute_statement(&statement, &mut db()).unwrap();

    assert_eq!(traced_off, plain);
    assert!(plain.iter().all(|l| !l.starts_with("[CTE]")), "{plain:#?}");
}

#[test]
fn trace_numa_query_sem_cte_nao_acrescenta_nada() {
    let statement = Statement::Query(simple(body("users", vec![], columns(&["name"]), None)));

    let traced = execute_statement_traced(&statement, &mut db(), true).unwrap();
    let plain = execute_statement(&statement, &mut db()).unwrap();

    assert_eq!(traced, plain);
}

// ----- JOIN: NULL do LEFT JOIN e nomes qualificados ----------------------

#[test]
fn left_join_preenche_sem_correspondencia_com_null() {
    // FROM usuarios LEFT JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto;
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Left, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    // Carlos não tem pedido: a célula fica vazia, a mesma convenção de NULL do CSV
    assert_eq!(result.rows.last().unwrap(), &vec!["Carlos".to_string(), String::new()]);
}

#[test]
fn where_sobre_coluna_nula_do_left_join_nao_quebra() {
    // Antes o LEFT JOIN preenchia com o texto "NULL" e isto dava
    // "Valor 'NULL' inválido para a coluna 'valor' (Float)".
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Left, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome"]),
        Some(cmp(id("valor"), Operator::Greater, Value::Int(100))),
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    // Carlos (valor NULL) fica de fora: NULL > 100 é desconhecido
    assert_eq!(first_column(&result), vec!["Ana", "Bruno"]);
}

#[test]
fn inner_join_com_nomes_qualificados() {
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Inner, "pedidos", "usuarios.id", "pedidos.usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(result.rows, vec![vec!["Ana", "Teclado"], vec!["Ana", "Mouse"], vec!["Bruno", "Monitor"]]);
}

#[test]
fn prefixo_escolhe_a_coluna_certa_quando_o_nome_se_repete() {
    // SELECT usuarios.id, pedidos.id: mesmo nome, colunas diferentes
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Inner, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["usuarios.id", "pedidos.id"]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    // a saída usa o nome sem prefixo, como no SQL
    assert_eq!(column_names(&result), vec!["id", "id"]);
    assert_eq!(result.rows, vec![vec!["1", "101"], vec!["1", "102"], vec!["2", "103"]]);
}

#[test]
fn join_entre_cte_e_tabela_com_prefixo() {
    // WITH a AS (FROM users SELECT name WHERE age < 25) FROM a JOIN users ON a.name = users.name SELECT a.name, age;
    let query = with(
        vec![cte("a", None, body("users", vec![], columns(&["name"]), Some(cmp(id("age"), Operator::Less, Value::Int(25)))))],
        body("a", vec![join(JoinType::Inner, "users", "a.name", "users.name")], columns(&["a.name", "age"]), None),
    );
    let result = execute_query(&query, &db()).unwrap();

    // uma linha por pessoa (antes, `ON name = name` dava produto cartesiano)
    assert_eq!(result.rows, vec![vec!["ana", "20"], vec!["bruno", "17"]]);
}

// ----- RIGHT e FULL JOIN ---------------------------------------------------

#[test]
fn right_join_mantem_linhas_da_direita_sem_correspondencia() {
    // FROM usuarios RIGHT JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto;
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Right, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    // Carlos (sem pedido) some; o pedido 104 (usuário 4 não existe) entra com nome NULL
    assert_eq!(
        result.rows,
        vec![vec!["Ana", "Teclado"], vec!["Ana", "Mouse"], vec!["Bruno", "Monitor"], vec!["", "Headset"]]
    );
}

#[test]
fn full_join_mantem_os_dois_lados() {
    // FROM usuarios FULL JOIN pedidos ON usuarios.id = usuario_id SELECT nome, produto;
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Full, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(
        result.rows,
        vec![
            vec!["Ana", "Teclado"],
            vec!["Ana", "Mouse"],
            vec!["Bruno", "Monitor"],
            vec!["Carlos", ""],
            vec!["", "Headset"],
        ]
    );
}

#[test]
fn right_join_equivale_ao_left_join_invertido() {
    // A RIGHT JOIN B tem as mesmas linhas que B LEFT JOIN A (só a ordem das colunas muda)
    let right = simple(body(
        "usuarios",
        vec![join(JoinType::Right, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let left = simple(body(
        "pedidos",
        vec![join(JoinType::Left, "usuarios", "usuarios.id", "usuario_id")],
        columns(&["nome", "produto"]),
        None,
    ));
    let db = db_all();
    let mut a = execute_query(&right, &db).unwrap().rows;
    let mut b = execute_query(&left, &db).unwrap().rows;
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn where_sobre_coluna_nula_do_right_join_nao_quebra() {
    // O pedido 104 tem `idade` NULL: NULL < 30 é desconhecido e a linha sai
    let query = simple(body(
        "usuarios",
        vec![join(JoinType::Right, "pedidos", "usuarios.id", "usuario_id")],
        columns(&["produto"]),
        Some(cmp(id("idade"), Operator::Less, Value::Int(30))),
    ));
    let result = execute_query(&query, &db_all()).unwrap();

    assert_eq!(first_column(&result), vec!["Teclado", "Mouse"]);
}