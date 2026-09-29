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
mini-sql/
|── generated/
│   └── código gerado pelo ANTLR
│
├── grammar/
│   └── SQL.g4
│
├── src/
│   │── lib.rs
│   └── main.rs
│    
│
├── tests/
│   └── parser_tests.rs
│
├── Cargo.toml  (Arquivo de configuração do projeto Rust)
├── Cargo.lock
└── README.md