// Interpretador: executa a AST (já verificada pelo type checker) sobre
// as tabelas carregadas em memória.

// As células ficam guardadas como texto em `Table::rows`. Na hora de
// avaliar o WHERE, cada célula é convertida para um valor tipado
// (`Cell`) usando o tipo que o `csv_loader` inferiu para a coluna.

// Célula vazia no CSV vira `Cell::Null` e nao quebra a consulta
//
// CTEs (`WITH nome AS (...)`) viram tabelas temporárias: existem só enquanto a
// query que as declarou é executada (ver `Env` e `run_query`).
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ast::{Condition, Describe, JoinType, Expr, ExprIn, Load, Operator, Query, QueryBody, SelectList, Statement, Value};
use crate::csv_loader::{load_csv, table_name_from_path};
use crate::table::{rename_columns, with_source, Column, DataType, Database, Table};

pub type ExecResult<T> = Result<T, String>;

type Truth = Option<bool>;

#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
}

/// Tabelas que um `FROM`/`JOIN` pode citar durante a execução de uma query.
///
/// É a mesma estrutura do `Env` do type checker, com duas camadas:
/// 1. `ctes`: as tabelas temporárias criadas pelas CTEs desta query (COM linhas);
/// 2. `database`: as tabelas carregadas por LOAD.
///
/// A busca olha primeiro as CTEs, igual ao type checker. Por isso uma query que
/// o type checker aceitou encontra, na execução, exatamente as mesmas tabelas.
///
/// Nada do Database é copiado: as tabelas carregadas são só lidas (`&`). As
/// tabelas temporárias pertencem ao `Env`, e quando ele sai de escopo, no fim
/// de `run_query`, o Rust as destrói. É isso que "descarta" uma CTE.
struct Env<'a> {
    database: &'a Database,
    ctes: HashMap<String, Table>,
}

impl<'a> Env<'a> {
    fn new(database: &'a Database) -> Self {
        Env { database, ctes: HashMap::new() }
    }

    /// Procura uma tabela: primeiro nas CTEs, depois no Database.
    fn lookup(&self, name: &str) -> ExecResult<&Table> {
        match self.ctes.get(name) {
            Some(table) => Ok(table),
            None => get_table(name, self.database),
        }
    }
}

// Statements

pub fn execute_statement(statement: &Statement, database: &mut Database) -> ExecResult<Vec<String>> {
    execute_statement_traced(statement, database, false)
}

/// Igual a `execute_statement`, mas com `trace = true` a saída de uma query
/// também mostra as tabelas temporárias das CTEs: quando cada uma é criada
/// (com o conteúdo dela) e quando são descartadas. Usado pelo `.trace` do terminal.
pub fn execute_statement_traced(statement: &Statement, database: &mut Database, trace: bool) -> ExecResult<Vec<String>> {
    match statement {
        Statement::Load(load) => Ok(vec![execute_load(load, database)?]),
        Statement::Describe(describe) => execute_describe(describe, database),
        Statement::Query(query) => {
            let mut log = Vec::new();
            let result = run_query(query, database, if trace { Some(&mut log) } else { None })?;
            log.extend(result.render());
            Ok(log)
        }
    }
}

/// LOAD 'arquivo.csv'; -> lê o CSV e registra a tabela com o nome do arquivo.
pub fn execute_load(load: &Load, database: &mut Database) -> ExecResult<String> {
    let name = table_name_from_path(&load.file)?;

    let table = load_csv(&load.file).map_err(|error| format!("Erro ao carregar '{}': {error}", load.file))?;

    let replaced = database.insert(name.clone(), table).is_some();

    Ok(format!(
        "Tabela '{name}' {}.",
        if replaced { "recarregada" } else { "carregada" },
    ))
}

/// DESCRIBE tabela; -> lista as colunas e seus tipos.
pub fn execute_describe(describe: &Describe, database: &Database) -> ExecResult<Vec<String>> {
    let table = get_table(&describe.table, database)?;

    let schema = Table {
        columns: vec![
            Column { name: "coluna".to_string(), data_type: DataType::String, source: None },
            Column { name: "tipo".to_string(), data_type: DataType::String, source: None },
        ],
        rows: table
            .columns
            .iter()
            .map(|column| vec![column.name.clone(), column.data_type.to_string()])
            .collect(),
    };

    let mut lines = schema.render();
    lines.pop(); // remove o "(n linhas)": aqui ele contaria colunas, não linhas
    lines.push(format!("({} colunas, {} linhas)", table.columns.len(), table.rows.len()));
    Ok(lines)
}

/// [WITH ...] FROM tabela SELECT ... WHERE ...; -> devolve uma nova tabela com o resultado.
pub fn execute_query(query: &Query, database: &Database) -> ExecResult<Table> {
    run_query(query, database, None)
}

/// Executa as CTEs em ordem, cada uma virando uma tabela temporária, e depois o corpo.
///
/// `WITH a AS (...), b AS (FROM a ...) FROM b ...`
///   1. executa o corpo de `a` e guarda o resultado como a tabela temporária `a`;
///   2. executa o corpo de `b`, que já enxerga `a`, e guarda como `b`;
///   3. executa o corpo principal, que enxerga `a` e `b`;
///   4. ao sair da função, `a` e `b` são descartadas junto com o `env`.
///
/// `trace`: quando presente, recebe as mensagens de criação e descarte das
/// tabelas temporárias (modo `.trace` do terminal).
fn run_query(query: &Query, database: &Database, mut trace: Option<&mut Vec<String>>) -> ExecResult<Table> {
    let mut env = Env::new(database);

    for cte in &query.ctes {
        let mut table = execute_body(&cte.body, &env)?;

        // Mesma renomeação do type checker (função compartilhada em table.rs).
        if let Some(names) = &cte.columns {
            rename_columns(&mut table.columns, names).map_err(|error| format!("A CTE '{}' {error}", cte.name))?;
        }

        if let Some(log) = trace.as_mut() {
            log.push(format!("[CTE] tabela temporária '{}' criada:", cte.name));
            log.extend(table.render());
        }

        env.ctes.insert(cte.name.clone(), table);
    }

    let result = execute_body(&query.body, &env)?;

    if let Some(log) = trace.as_mut() {
        if !query.ctes.is_empty() {
            let names: Vec<&str> = query.ctes.iter().map(|cte| cte.name.as_str()).collect();
            log.push(format!("[CTE] tabelas temporárias descartadas: {}", names.join(", ")));
        }
    }

    Ok(result)
}

/// Executa `FROM ... JOIN ... SELECT ... WHERE ...` (o corpo de uma query ou de uma CTE).
fn execute_body(query: &QueryBody, env: &Env) -> ExecResult<Table> {
    // Começa com a tabela principal do FROM (pode ser uma tabela carregada ou uma CTE),
    // com as colunas marcadas pelo nome dela, igual ao type checker.
    let base = env.lookup(&query.table)?;
    let mut current_table = Table { columns: with_source(&base.columns, &query.table), rows: base.rows.clone() };

    // Aplica cada JOIN em sequência
    for join in &query.joins {
        let right_table = env.lookup(&join.table)?;
        current_table = execute_join(&current_table, right_table, &join.table, &join.join_type, &join.condition)?;
    }

    // Projeção: índices das colunas pedidas no SELECT (sobre a tabela já com os joins aplicados)
    let selected: Vec<usize> = match &query.select {
        SelectList::All => (0..current_table.columns.len()).collect(),
        SelectList::Columns(names) => names
            .iter()
            .map(|name| column_index(name, &current_table))
            .collect::<ExecResult<_>>()?,
    };

    // Seleção: mantém só as linhas em que o WHERE é verdadeiro.
    let mut rows = Vec::new();
    for row in &current_table.rows {
        let keep = match &query.condition {
            Some(condition) => evaluate_condition(condition, row, &current_table)? == Some(true),
            None => true,
        };

        if keep {
            rows.push(selected.iter().map(|&i| row[i].clone()).collect());
        }
    }

    let columns = selected.iter().map(|&i| current_table.columns[i].clone()).collect();

    Ok(Table { columns, rows })
}

fn execute_join(
    left_table: &Table,
    right_table: &Table,
    right_name: &str,
    join_type: &JoinType,
    join_cond: &Expr,
) -> ExecResult<Table> {
    // Combina as colunas das duas tabelas. As da direita são marcadas com o
    // nome da tabela do JOIN, para `pedidos.id` e `usuarios.id` serem distintas.
    let mut columns = left_table.columns.clone();
    columns.extend(with_source(&right_table.columns, right_name));

    // Cria a tabela temporária para o escopo das colunas combinadas
    let combined_table = Table {
        columns: columns.clone(),
        rows: Vec::new(),
    };

    // Quais lados mantêm as linhas sem correspondência:
    //   INNER: nenhum | LEFT: esquerda | RIGHT: direita | FULL: os dois
    let keep_left = matches!(join_type, JoinType::Left | JoinType::Full);
    let keep_right = matches!(join_type, JoinType::Right | JoinType::Full);

    // Linhas sem par são completadas com strings VAZIAS do outro lado: é a mesma
    // convenção do CSV, e é ela que o `parse_cell` reconhece como NULL. O texto
    // "NULL" seria lido como um valor e quebraria `WHERE valor > 100`
    let left_nulls = vec![String::new(); left_table.columns.len()];
    let right_nulls = vec![String::new(); right_table.columns.len()];

    // Marca as linhas da direita que casaram com alguma da esquerda (para o RIGHT/FULL).
    let mut right_matched = vec![false; right_table.rows.len()];

    let mut rows = Vec::new();

    // Nested loop: cada linha da esquerda contra cada linha da direita.
    for left_row in &left_table.rows {
        let mut left_matched = false;

        for (j, right_row) in right_table.rows.iter().enumerate() {
            let mut combined_row = left_row.clone();
            combined_row.extend(right_row.iter().cloned());

            if evaluate_expr(join_cond, &combined_row, &combined_table)? == Some(true) {
                rows.push(combined_row);
                left_matched = true;
                right_matched[j] = true;
            }
        }

        // LEFT/FULL: a linha da esquerda sem par entra com a direita NULL.
        if keep_left && !left_matched {
            let mut combined_row = left_row.clone();
            combined_row.extend(right_nulls.iter().cloned());
            rows.push(combined_row);
        }
    }

    // RIGHT/FULL: as linhas da direita que não casaram com nenhuma da esquerda
    // entram no fim, com a esquerda NULL. As colunas continuam na mesma ordem
    // (esquerda, depois direita), só os valores da esquerda ficam vazios.
    if keep_right {
        for (right_row, matched) in right_table.rows.iter().zip(&right_matched) {
            if !matched {
                let mut combined_row = left_nulls.clone();
                combined_row.extend(right_row.iter().cloned());
                rows.push(combined_row);
            }
        }
    }

    Ok(Table { columns, rows })
}

// Avaliação do WHERE

/// Avalia uma condição para uma linha.
fn evaluate_condition(condition: &Condition, row: &[String], table: &Table) -> ExecResult<Truth> {
    Ok(match condition {
        Condition::Expr(expr) => evaluate_expr(expr, row, table)?,
        Condition::ExprIn(expr_in) => evaluate_in(expr_in, row, table)?,
        Condition::Parens(inner) => evaluate_condition(inner, row, table)?,
        Condition::Not(inner) => evaluate_condition(inner, row, table)?.map(|b| !b),

        // AND: falso se algum lado é falso; senão desconhecido se algum é desconhecido.
        Condition::And(left, right) => {
            match (evaluate_condition(left, row, table)?, evaluate_condition(right, row, table)?) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            }
        }

        // OR: verdadeiro se algum lado é verdadeiro; senão desconhecido se algum é desconhecido.
        Condition::Or(left, right) => {
            match (evaluate_condition(left, row, table)?, evaluate_condition(right, row, table)?) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            }
        }
    })
}

/// `left op right`.
fn evaluate_expr(expr: &Expr, row: &[String], table: &Table) -> ExecResult<Truth> {
    let left = evaluate_value(&expr.left, row, table)?;
    let right = evaluate_value(&expr.right, row, table)?;
    compare(&left, &expr.op, &right).map_err(|e| format!("{e} em `{expr}`"))
}

/// `value IN (v1, v2, ...)`: equivale a `value = v1 OR value = v2 OR ...`.
fn evaluate_in(expr_in: &ExprIn, row: &[String], table: &Table) -> ExecResult<Truth> {
    let value = evaluate_value(&expr_in.value, row, table)?;

    let mut result = Some(false);
    for item in &expr_in.values {
        let item = evaluate_value(item, row, table)?;
        match compare(&value, &Operator::Equal, &item)? {
            Some(true) => return Ok(Some(true)),
            Some(false) => {}
            None => result = None,
        }
    }
    Ok(result)
}

//mais tipos, antes só int64
fn compare(left: &Cell, op: &Operator, right: &Cell) -> ExecResult<Truth> {
    let ordering = match (left, right) {
        (Cell::Null, _) | (_, Cell::Null) => return Ok(None),

        (Cell::Int(a), Cell::Int(b)) => a.cmp(b),
        (Cell::Int(a), Cell::Float(b)) => cmp_float(*a as f64, *b)?,
        (Cell::Float(a), Cell::Int(b)) => cmp_float(*a, *b as f64)?,
        (Cell::Float(a), Cell::Float(b)) => cmp_float(*a, *b)?,
        (Cell::Str(a), Cell::Str(b)) => a.cmp(b),

        (Cell::Bool(a), Cell::Bool(b)) => {
            return match op {
                Operator::Equal => Ok(Some(a == b)),
                Operator::NotEqual => Ok(Some(a != b)),
                _ => Err(format!("Operador `{op}` não se aplica a Boolean")),
            };
        }

        // O type checker impede que isso aconteça; fica como proteção.
        _ => return Err(format!("Não é possível comparar {left:?} com {right:?}")),
    };

    Ok(Some(match op {
        Operator::Equal => ordering == Ordering::Equal,
        Operator::NotEqual => ordering != Ordering::Equal,
        Operator::Less => ordering == Ordering::Less,
        Operator::LessEqual => ordering != Ordering::Greater,
        Operator::Greater => ordering == Ordering::Greater,
        Operator::GreaterEqual => ordering != Ordering::Less,
    }))
}

fn cmp_float(a: f64, b: f64) -> ExecResult<Ordering> {
    a.partial_cmp(&b).ok_or_else(|| "Comparação com NaN".to_string())
}

/// Valor de um literal, ou da célula da linha atual se for uma coluna.
fn evaluate_value(value: &Value, row: &[String], table: &Table) -> ExecResult<Cell> {
    Ok(match value {
        Value::Int(v) => Cell::Int(*v),
        Value::Float(v) => Cell::Float(*v),
        Value::String(v) => Cell::Str(v.clone()),
        Value::Boolean(v) => Cell::Bool(*v),
        Value::Id(name) => {
            let index = column_index(name, table)?;
            parse_cell(&row[index], &table.columns[index])?
        }
    })
}

/// Converte o texto de uma célula para o tipo da coluna.
fn parse_cell(raw: &str, column: &Column) -> ExecResult<Cell> {
    if raw.is_empty() {
        return Ok(Cell::Null);
    }

    let invalid = || format!("Valor '{raw}' inválido para a coluna '{}' ({})", column.name, column.data_type);

    Ok(match column.data_type {
        DataType::Int => Cell::Int(raw.parse().map_err(|_| invalid())?),
        DataType::Float => Cell::Float(raw.parse().map_err(|_| invalid())?),
        DataType::Boolean => Cell::Bool(raw.eq_ignore_ascii_case("true")),
        DataType::String => Cell::Str(raw.to_string()),
    })
}

// Auxiliares

fn get_table<'a>(name: &str, database: &'a Database) -> ExecResult<&'a Table> {
    database.get(name).ok_or_else(|| format!("Tabela '{name}' não encontrada"))
}

/// Posição de uma coluna (`coluna` ou `tabela.coluna`): mesma regra do type checker.
fn column_index(name: &str, table: &Table) -> ExecResult<usize> {
    table.resolve(name)
}