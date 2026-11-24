// Testes de ponta a ponta: texto SQL -> parser -> type checker -> interpretador.

use mini_sql::pipeline::{run_pipeline, run_pipeline_traced};
use mini_sql::table::Database;

fn run(sql: &str) -> Result<Vec<String>, String> {
    let mut db = Database::new();
    run_pipeline(sql, &mut db)
}

#[test]
fn load_e_consulta_no_mesmo_programa() {
    let out = run("LOAD 'test_data/users.csv'; FROM users SELECT name WHERE age < 18;").unwrap();

    assert_eq!(out, vec!["Tabela 'users' carregada.", "name", "-----", "bruno", "(1 linha)"]);
}

#[test]
fn load_persiste_entre_entradas() {
    // Simula o REPL: cada linha é uma chamada separada, com o mesmo banco.
    let mut db = Database::new();
    run_pipeline("LOAD 'test_data/alunos.csv';", &mut db).unwrap();
    let out = run_pipeline("FROM alunos SELECT nome WHERE idade >= 20;", &mut db).unwrap();

    assert_eq!(out, vec!["nome", "-----", "Ana", "Bruno", "(2 linhas)"]);
}

#[test]
fn describe() {
    let out = run("LOAD 'test_data/alunos.csv'; DESCRIBE alunos;").unwrap();

    assert!(out.contains(&"idade  | Int".to_string()), "{out:#?}");
}

#[test]
fn exemplo_do_readme() {
    let out = run(
        "LOAD 'test_data/users.csv';
         FROM users SELECT name, credit
         WHERE type IN ('admin', 'editor') AND active = TRUE AND credit >= 150.50;",
    )
    .unwrap();

    assert_eq!(out[1..], ["name  | credit", "------+-------", "ana   | 200.75", "elisa | 310.0", "(2 linhas)"][..]);
}

#[test]
fn consulta_sem_load_e_erro_semantico() {
    let err = run("FROM users SELECT *;").unwrap_err();
    assert!(err.starts_with("Erro semântico"), "{err}");
}

#[test]
fn tipo_incompativel_e_erro_semantico() {
    let err = run("LOAD 'test_data/users.csv'; FROM users SELECT name WHERE age = 'vinte';").unwrap_err();
    assert!(err.starts_with("Erro semântico"), "{err}");
    assert!(err.contains("age = 'vinte'"), "{err}");
}

#[test]
fn arquivo_inexistente_e_erro_de_execucao() {
    let err = run("LOAD 'test_data/nao_existe.csv';").unwrap_err();
    assert!(err.starts_with("Erro de execução"), "{err}");
}

#[test]
fn erro_de_sintaxe() {
    let err = run("SELECT name FROM users;").unwrap_err();
    assert!(err.starts_with("Erro de sintaxe"), "{err}");
}

// O parser do ANTLR se recupera de erros e devolve uma árvore "consertada".
// Os testes abaixo garantem que essa árvore NÃO é executada.

#[test]
fn sem_ponto_e_virgula_e_erro_de_sintaxe() {
    // antes: o ANTLR inseria o `;` e a consulta executava normalmente
    let err = run("LOAD 'test_data/users.csv'; FROM users SELECT name").unwrap_err();
    assert!(err.starts_with("Erro de sintaxe"), "{err}");
}

#[test]
fn valor_faltando_nao_e_inventado() {
    // antes: o ANTLR inventava um valor e o erro aparecia como semântico
    // ("... o literal FALSE é Boolean")
    let err = run("LOAD 'test_data/users.csv'; FROM users SELECT name WHERE age >= ;").unwrap_err();
    assert!(err.starts_with("Erro de sintaxe"), "{err}");
}

#[test]
fn erro_de_sintaxe_informa_linha_e_coluna() {
    let err = run("LOAD 'test_data/users.csv';\nFROM users SELECT;").unwrap_err();
    assert!(err.contains("linha 2, coluna 18"), "{err}");
}

#[test]
fn erro_de_sintaxe_num_statement_impede_todos() {
    // O texto inteiro é analisado antes de qualquer execução: com um erro no
    // segundo statement, nem o LOAD do primeiro roda.
    let mut db = Database::new();
    let err = run_pipeline("LOAD 'test_data/alunos.csv'; FROM alunos SELECT nome WHERE ;", &mut db).unwrap_err();

    assert!(err.starts_with("Erro de sintaxe"), "{err}");
    assert!(db.is_empty(), "o LOAD não deveria ter executado");
}

// ----- CTEs, JOIN e nomes qualificados (ponta a ponta) -------------------

const LOAD_JOIN: &str = "LOAD 'test_data/usuarios.csv'; LOAD 'test_data/pedidos.csv';";

#[test]
fn cte_ponta_a_ponta() {
    let out = run(
        "LOAD 'test_data/users.csv';
         WITH adultos (nome, anos) AS (FROM users SELECT name, age WHERE age >= 18)
         FROM adultos SELECT nome WHERE anos > 30;",
    )
    .unwrap();

    assert_eq!(out[1..], ["nome", "-----", "carla", "diego", "(2 linhas)"][..]);
}

#[test]
fn cte_nao_existe_na_entrada_seguinte() {
    // Simula o REPL: a CTE de uma linha não sobrevive para a próxima.
    let mut db = Database::new();
    run_pipeline("LOAD 'test_data/users.csv';", &mut db).unwrap();
    run_pipeline("WITH adultos AS (FROM users SELECT name WHERE age >= 18) FROM adultos SELECT name;", &mut db).unwrap();

    let err = run_pipeline("FROM adultos SELECT name;", &mut db).unwrap_err();
    assert!(err.starts_with("Erro semântico"), "{err}");
    assert!(err.contains("Tabela 'adultos' não encontrada"), "{err}");
}

#[test]
fn erro_numa_cte_e_erro_semantico() {
    let err = run("LOAD 'test_data/users.csv'; WITH a (x, y) AS (FROM users SELECT name) FROM a SELECT x;").unwrap_err();

    assert!(err.starts_with("Erro semântico"), "{err}");
    assert!(err.contains("declara 2 coluna(s)"), "{err}");
}

#[test]
fn trace_mostra_as_tabelas_temporarias() {
    let mut db = Database::new();
    let out = run_pipeline_traced(
        "LOAD 'test_data/users.csv';
         WITH adultos AS (FROM users SELECT name WHERE age >= 35) FROM adultos SELECT name;",
        &mut db,
        true,
    )
    .unwrap();

    assert_eq!(
        out,
        vec![
            "Tabela 'users' carregada.",
            "[CTE] tabela temporária 'adultos' criada:",
            "name",
            "-----",
            "carla",
            "diego",
            "(2 linhas)",
            "[CTE] tabelas temporárias descartadas: adultos",
            "name",
            "-----",
            "carla",
            "diego",
            "(2 linhas)",
        ]
    );
}

#[test]
fn join_com_nomes_qualificados_ponta_a_ponta() {
    let out = run(&format!(
        "{LOAD_JOIN} FROM usuarios JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT nome, produto WHERE valor > 100;"
    ))
    .unwrap();

    assert_eq!(out[2..], ["nome  | produto", "------+--------", "Ana   | Teclado", "Bruno | Monitor", "(2 linhas)"][..]);
}

#[test]
fn coluna_ambigua_e_erro_semantico() {
    let err = run(&format!("{LOAD_JOIN} FROM usuarios JOIN pedidos ON id = usuario_id SELECT nome;")).unwrap_err();

    assert!(err.starts_with("Erro semântico"), "{err}");
    assert!(err.contains("Coluna 'id' é ambígua"), "{err}");
}

#[test]
fn left_join_com_where_na_coluna_nula() {
    // regressão: o LEFT JOIN preenchia com o texto "NULL" e isto dava erro de execução
    let out = run(&format!(
        "{LOAD_JOIN} FROM usuarios LEFT JOIN pedidos ON usuarios.id = usuario_id SELECT nome WHERE valor > 100;"
    ))
    .unwrap();

    assert_eq!(out[2..], ["nome", "-----", "Ana", "Bruno", "(2 linhas)"][..]);
}

#[test]
fn cte_sobre_left_join_com_lista_de_colunas() {
    let out = run(&format!(
        "{LOAD_JOIN}
         WITH compras (uid, nome, idade, pid, usuario_id, produto, valor) AS
              (FROM usuarios LEFT JOIN pedidos ON usuarios.id = usuario_id SELECT *),
              caras AS (FROM compras SELECT nome, produto WHERE valor >= 150)
         FROM caras SELECT nome, produto;"
    ))
    .unwrap();

    // Carlos entra em `compras` (LEFT JOIN) com valor NULL e sai em `caras`
    assert_eq!(out[2..], ["nome  | produto", "------+--------", "Ana   | Teclado", "Bruno | Monitor", "(2 linhas)"][..]);
}
