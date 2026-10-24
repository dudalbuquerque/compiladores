# Compiladores

## MiniSQL

Projeto desenvolvido para a disciplina de **Compiladores**, com o objetivo de implementar uma linguagem baseada em um subconjunto da linguagem SQL.

## Objetivo

Desenvolver uma implementação de uma linguagem SQL simplificada, utilizando **Rust** e **ANTLR**, aplicando conceitos de construção de compiladores, como análise léxica e análise sintática.

## Tecnologias

- **Rust** — linguagem de programação utilizada para implementar o projeto.
- **Cargo** — ferramenta de gerenciamento de projetos Rust. É utilizada para criar, compilar, executar, testar o projeto e gerenciar suas dependências.
- **ANTLR4** — ferramenta utilizada para definir uma gramática e gerar componentes de análise léxica e sintática.
- **antlr-rust-runtime / antlr-rust-codegen** — implementação independente em Rust utilizada para executar os reconhecedores gerados e gerar código Rust a partir da gramática `.g4`.

## Escopo

A linguagem será baseada em um subconjunto da linguagem SQL.

O escopo será definido antes da implementação da gramática, determinando quais comandos, cláusulas e operadores serão suportados.

## Estrutura planejada

```text
mini_sql/
├── generated/
│   └── código gerado pelo ANTLR
│
├── grammar/
│   └── miniSQL.g4
│
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── ast.rs
│   ├── ast_builder.rs
│   ├── ast_printer.rs
│   ├── csv_loader.rs
│   ├── table.rs
│   └── executor.rs
│
├── tests/
│   ├── ast_test.rs
│   ├── csv_loader_test.rs
│   ├── parser_error_test.rs
│   └── parser_success_test.rs
│
├── test_data/
│   └── alunos.csv
│
├── Cargo.toml
├── Cargo.lock
└── README.md


| Arquivo | Responsabilidade |
|---|---|
| `lib.rs` | Expõe os módulos e integra o código gerado pelo ANTLR. |
| `main.rs` | Recebe comandos do terminal e apresenta os resultados. |
| `ast.rs` | Define as estruturas da AST. |
| `ast_builder.rs` | Constrói a AST a partir da análise sintática. |
| `ast_printer.rs` | Exibe a AST para facilitar a visualização e depuração. |
| `csv_loader.rs` | Carrega os dados dos arquivos CSV. |
| `table.rs` | Define banco de dados, tabelas, colunas e tipos. |
| `executor.rs` | Executa comandos e consultas, incluindo a avaliação do `WHERE`. |
| `tests/` | Contém testes de AST, parser e carregamento de CSV. |
| `test_data/` | Guarda os CSVs utilizados nos testes manuais. |