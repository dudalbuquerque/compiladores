// Roda sobre a AST, ANTES da execução, e rejeita programas que são

// - tabela inexistente (`FROM xyz` sem `LOAD` antes);
// - coluna inexistente no SELECT ou no WHERE;
// - comparação entre tipos incompatíveis (`idade = 'vinte'`);
// - operador de ordem em tipo sem ordem (`ativo > TRUE`);
// - lista do IN com tipos misturados (`idade IN (1, 'a')`);
// - LOAD de arquivo que não é `.csv` ou cujo nome não vira um identificador;
// - CTE com nome repetido no mesmo WITH;
// - CTE cuja lista de colunas `(c1, c2)` não bate com o SELECT do corpo;
// - CTE que produz duas colunas com o mesmo nome (ex.: `SELECT *` de um JOIN).
//
// O ambiente de tipos é o esquema das tabelas já carregadas: nome da
// tabela -> colunas e seus tipos (`table::Database`). Os tipos das colunas
// são inferidos pelo `csv_loader` no momento do LOAD.
//
// Dentro de uma query com WITH, o ambiente ganha uma segunda camada: as CTEs
// já verificadas (`Env`). Elas só existem durante a checagem daquela query.


//ponto de entrada é o statement, que pode ser LOAD, DESCRIBE ou SELECT. Cada um tem sua função de checagem.

use std::collections::{HashMap, HashSet};

use crate::ast::{Condition, Cte, Describe, Expr, ExprIn, Join, Load, Operator, Query, QueryBody, SelectList, Statement, Value};
use crate::csv_loader::table_name_from_path;
use crate::table::{rename_columns, with_source, Column, DataType, Database, Table};

pub type TypeResult<T> = Result<T, String>;

/// Ambiente de tipos de uma query: as tabelas que um `FROM`/`JOIN` pode citar.
///
/// São duas camadas, como escopos aninhados numa linguagem de programação:
/// 1. `ctes`: as CTEs já verificadas desta query (só esquema, sem linhas);
/// 2. `database`: as tabelas carregadas por LOAD.
///
/// A busca olha primeiro as CTEs, então uma CTE com o nome de uma tabela
/// carregada a esconde, como uma variável local esconde uma global.
///
/// O `Env` é local a `check_query` e só lê o `Database` (`&`, não `&mut`):
/// quando a query termina, as CTEs somem junto com ele.
struct Env<'a> {
    database: &'a Database,
    ctes: HashMap<String, Table>,
}

impl<'a> Env<'a> {
    /// Ambiente sem CTEs: só as tabelas carregadas.
    fn new(database: &'a Database) -> Self {
        Env { database, ctes: HashMap::new() }
    }

    /// Procura uma tabela: primeiro nas CTEs, depois no Database.
    fn lookup(&self, name: &str) -> TypeResult<&Table> {
        match self.ctes.get(name) {
            Some(table) => Ok(table),
            None => lookup_table(name, self.database),
        }
    }

    /// Torna uma CTE visível para o que vem depois dela (próximas CTEs e corpo).
    /// Duas CTEs com o mesmo nome no mesmo WITH são erro, como no SQL.
    fn define_cte(&mut self, name: &str, columns: Vec<Column>) -> TypeResult<()> {
        if self.ctes.contains_key(name) {
            return Err(format!("CTE '{name}' definida mais de uma vez no mesmo WITH"));
        }
        self.ctes.insert(name.to_string(), Table { columns, rows: vec![] });
        Ok(())
    }
}

/// Ponto de entrada: verifica um statement contra as tabelas carregadas.
pub fn check_statement(statement: &Statement, database: &Database) -> TypeResult<()> {
    match statement {
        Statement::Load(load) => check_load(load),
        Statement::Describe(describe) => check_describe(describe, database),
        Statement::Query(query) => check_query(query, database),
    }
}

/// LOAD: o arquivo precisa ser `.csv` e o nome precisa virar um identificador.
/// A existência do arquivo só é verificada na execução 
pub fn check_load(load: &Load) -> TypeResult<()> {
    if !load.file.to_ascii_lowercase().ends_with(".csv") {
        return Err(format!("LOAD espera um arquivo .csv, recebeu '{}'", load.file));
    }
    table_name_from_path(&load.file)?;
    Ok(())
}

/// DESCRIBE: a tabela precisa existir.
pub fn check_describe(describe: &Describe, database: &Database) -> TypeResult<()> {
    lookup_table(&describe.table, database)?;
    Ok(())
}

/// Query: verifica as CTEs (em ordem) e depois o corpo.
///
/// `WITH a AS (...), b AS (...) FROM b ...`
///   - `a` é verificada vendo só o Database;
///   - `b` é verificada vendo o Database + `a`;
///   - o corpo é verificado vendo o Database + `a` + `b`.
///
/// Como cada CTE só entra no ambiente DEPOIS de verificada, três regras do
/// SQL não recursivo saem de graça desta ordem:
///   - uma CTE só enxerga as anteriores, nunca as seguintes;
///   - `WITH a AS (FROM a ...)` se refere à tabela `a` do Database, não a si mesma;
///   - as CTEs somem ao fim da query, porque o `env` é uma variável local.
pub fn check_query(query: &Query, database: &Database) -> TypeResult<()> {
    let mut env = Env::new(database);

    for cte in &query.ctes {
        let columns = check_cte(cte, &env)?;
        env.define_cte(&cte.name, columns)?;
    }

    check_query_body(&query.body, &env)?;
    Ok(())
}

/// Verifica uma CTE e devolve o esquema com que ela fica visível.
///
/// Passos:
/// 1. verifica o corpo e obtém as colunas que ele produz;
/// 2. se houver lista `nome (c1, c2) AS (...)`, renomeia as colunas;
/// 3. garante que não sobrou coluna com nome repetido.
fn check_cte(cte: &Cte, env: &Env) -> TypeResult<Vec<Column>> {
    // 1. Esquema do corpo. O prefixo diz em qual CTE está o erro: sem ele,
    //    "Coluna 'x' não existe" num WITH com várias CTEs seria difícil de achar.
    let mut columns = check_query_body(&cte.body, env)
        .map_err(|error| format!("Na CTE '{}': {error}", cte.name))?;

    // 2. Lista de colunas: renomeia POSIÇÃO a posição e mantém os tipos.
    //    A função é compartilhada com o interpretador (table::rename_columns),
    //    para as duas fases chegarem ao mesmo esquema.
    if let Some(names) = &cte.columns {
        rename_columns(&mut columns, names).map_err(|error| format!("A CTE '{}' {error}", cte.name))?;
    }

    // 3. Colunas repetidas. Acontece com `SELECT *` de um JOIN em que as duas
    //    tabelas têm `id`. Uma coluna repetida deixaria `FROM cte SELECT id`
    //    ambíguo, então recusamos aqui e indicamos a saída: a lista de colunas.
    //    Esta checagem vem DEPOIS da renomeação, porque a lista é justamente o
    //    jeito de resolver a repetição (e também pega `cte (a, a) AS ...`).
    let mut seen = HashSet::new();
    for column in &columns {
        if !seen.insert(column.name.as_str()) {
            return Err(format!(
                "A CTE '{}' produz a coluna '{}' mais de uma vez; dê nomes distintos com \
                 {}(coluna1, coluna2, ...) AS (...)",
                cte.name, column.name, cte.name,
            ));
        }
    }

    Ok(columns)
}

/// Verifica `FROM ... JOIN ... SELECT ... WHERE ...` e devolve o ESQUEMA do
/// resultado: as colunas que essa consulta produz, com seus tipos.
///
/// É a ideia clássica de type checker: verificar uma expressão e devolver o
/// tipo dela. Aqui o "tipo" de uma consulta é a lista de colunas da saída.
/// É isso que permite usar uma CTE como tabela sem executá-la.
fn check_query_body(body: &QueryBody, env: &Env) -> TypeResult<Vec<Column>> {
    let base_table = env.lookup(&body.table)?;

    // Escopo de colunas: começa com as colunas da tabela do FROM, marcadas
    // com o nome dela (é o que permite escrever `usuarios.id`)...
    let mut scope_columns = with_source(&base_table.columns, &body.table);

    // ...e cada JOIN acrescenta as colunas da tabela dele.
    check_joins(&body.joins, env, &mut scope_columns)?;

    // Tabela só de esquema com todas as colunas visíveis (FROM + JOINs).
    let scope_table = Table { columns: scope_columns, rows: vec![] };

    // O WHERE pode usar qualquer coluna do escopo, mesmo fora do SELECT.
    if let Some(condition) = &body.condition {
        check_condition(condition, &scope_table)?;
    }

    // Esquema de saída: `*` devolve o escopo inteiro; uma lista devolve só
    // as colunas pedidas, na ordem pedida, com o tipo que já tinham.
    let output = match &body.select {
        SelectList::All => scope_table.columns,
        SelectList::Columns(names) => names
            .iter()
            .map(|name| {
                // `SELECT pedidos.id` produz uma coluna chamada `id` (sem o prefixo),
                // como no SQL. Por isso `SELECT usuarios.id, pedidos.id` dentro de
                // uma CTE dá coluna repetida e pede a lista de nomes.
                let index = scope_table.resolve(name)?;
                Ok(scope_table.columns[index].clone())
            })
            .collect::<TypeResult<Vec<_>>>()?,
    };

    Ok(output)
}

fn check_joins(
    joins: &[Join],
    env: &Env,
    scope_columns: &mut Vec<Column>,
) -> TypeResult<()> {
    for join in joins {
        let join_table = env.lookup(&join.table)?;

        // O tipo do JOIN (INNER/LEFT/RIGHT/FULL) não muda o esquema: as colunas
        // são as mesmas, na mesma ordem. Só muda quais linhas existem e onde
        // aparece NULL, e isso é assunto do interpretador.

        // Adiciona as colunas da tabela atual do join ao escopo acumulado,
        // marcadas com o nome dela (`pedidos.id` ≠ `usuarios.id`)
        scope_columns.extend(with_source(&join_table.columns, &join.table));

        // Cria o escopo temporário acumulado até este join para validar a condição ON
        let current_scope_table = Table {
            columns: scope_columns.clone(),
            rows: vec![],
        };

        // Valida a expressão do ON com base em tudo o que está visível até agora
        check_expr(&join.condition, &current_scope_table)?;
    }

    Ok(())
}

/// Verifica recursivamente uma condição do WHERE.
pub fn check_condition(condition: &Condition, table: &Table) -> TypeResult<()> {
    match condition {
        Condition::Expr(expr) => check_expr(expr, table),
        Condition::ExprIn(expr_in) => check_expr_in(expr_in, table),

        Condition::And(left, right) | Condition::Or(left, right) => {
            check_condition(left, table)?;
            check_condition(right, table)
        }

        Condition::Not(inner) | Condition::Parens(inner) => check_condition(inner, table),
    }
}

fn check_expr(expr: &Expr, table: &Table) -> TypeResult<()> {
    let left_type = type_of(&expr.left, table)?;
    let right_type = type_of(&expr.right, table)?;

    if !left_type.is_compatible_with(&right_type) {
        return Err(format!(
            "Tipos incompatíveis em `{expr}`: {} é {left_type} e {} é {right_type}",
            describe_value(&expr.left),
            describe_value(&expr.right),
        ));
    }

    let is_order_op = matches!(
        expr.op,
        Operator::Less | Operator::LessEqual | Operator::Greater | Operator::GreaterEqual
    );

    if is_order_op && !left_type.is_ordered() {
        return Err(format!(
            "Operador `{}` não se aplica ao tipo {left_type} em `{expr}` (use = ou !=)",
            expr.op
        ));
    }

    Ok(())
}

/// Pertinência `value IN (v1, v2, ...)`: cada item segue a regra do `=`.
fn check_expr_in(expr_in: &ExprIn, table: &Table) -> TypeResult<()> {
    let value_type = type_of(&expr_in.value, table)?;

    for item in &expr_in.values {
        let item_type = type_of(item, table)?;

        if !value_type.is_compatible_with(&item_type) {
            return Err(format!(
                "Tipos incompatíveis no IN: {} é {value_type}, mas o item {item} é {item_type}",
                describe_value(&expr_in.value),
            ));
        }
    }

    Ok(())
}

/// Tipo de um valor: literal tem tipo fixo; identificador tem o tipo da coluna.
fn type_of(value: &Value, table: &Table) -> TypeResult<DataType> {
    match value {
        Value::Id(name) => Ok(lookup_column(name, table)?.clone()),
        Value::Int(_) => Ok(DataType::Int),
        Value::Float(_) => Ok(DataType::Float),
        Value::String(_) => Ok(DataType::String),
        Value::Boolean(_) => Ok(DataType::Boolean),
    }
}

fn lookup_table<'a>(name: &str, database: &'a Database) -> TypeResult<&'a Table> {
    database.get(name).ok_or_else(|| {
        format!("Tabela '{name}' não encontrada (use LOAD 'arquivo.csv'; antes de consultá-la)")
    })
}

/// Tipo de uma coluna (`coluna` ou `tabela.coluna`) no escopo da query.
/// A resolução do nome, inclusive o erro de nome ambíguo, é `Table::resolve`,
/// a mesma usada pelo interpretador.
fn lookup_column<'a>(name: &str, table: &'a Table) -> TypeResult<&'a DataType> {
    let index = table.resolve(name)?;
    Ok(&table.columns[index].data_type)
}

/// "a coluna 'idade'" ou "o literal 'vinte'", para as mensagens de erro.
fn describe_value(value: &Value) -> String {
    match value {
        Value::Id(name) => format!("a coluna '{name}'"),
        literal => format!("o literal {literal}"),
    }
}