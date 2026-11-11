// Testes de ponta a ponta: texto SQL -> parser -> type checker -> interpretador.

use mini_sql::pipeline::run_pipeline;
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