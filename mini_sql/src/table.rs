use std::collections::HashMap;
use std::fmt;

pub type Database = HashMap<String, Table>;

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    /// De qual tabela da query a coluna veio (o nome usado no FROM/JOIN).
    /// Só faz sentido dentro de uma query: é preenchido por `with_source`
    /// quando o escopo é montado, e é o que permite resolver `tabela.coluna`
    /// e detectar nomes ambíguos. Numa tabela carregada do CSV, é `None`.
    pub source: Option<String>,
}

/// Cópia das colunas marcadas como vindas de `source`.
/// Usado ao montar o escopo: `FROM usuarios JOIN pedidos ...` marca as colunas
/// de cada tabela com o nome dela. Uma CTE é marcada com o nome da CTE.
pub fn with_source(columns: &[Column], source: &str) -> Vec<Column> {
    columns
        .iter()
        .map(|column| Column { source: Some(source.to_string()), ..column.clone() })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    String,
    Int,
    Float,
    Boolean,
}

//A vantagem de isso morar no tipo é que o type checker só pergunta “esses tipos são compatíveis?”, sem repetir a regra

impl DataType {
    /// Int e Float podem ser comparados entre si (o Int é promovido a Float).
    pub fn is_numeric(&self) -> bool {
        matches!(self, DataType::Int | DataType::Float)
    }

    /// Tipos que aceitam <, <=, > e >=.
    pub fn is_ordered(&self) -> bool {
        matches!(self, DataType::Int | DataType::Float | DataType::String)
    }

    /// Dois tipos são compatíveis se forem iguais ou ambos numéricos.
    pub fn is_compatible_with(&self, other: &DataType) -> bool {
        self == other || (self.is_numeric() && other.is_numeric())
    }
}

//melhora as mensagens de erro 
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DataType::String => "String",
            DataType::Int => "Int",
            DataType::Float => "Float",
            DataType::Boolean => "Boolean",
        };
        write!(f, "{name}")
    }
}

//metodso que estavam sendo chamados partes diferentes do código ( typechecker e interpretador)
//render transforma uma tabela em linhas de texto alinhadas, com cabeçalho e contagem de linhas.
/// Renomeia colunas posição a posição, mantendo os tipos:
/// `adultos (nome, anos) AS (FROM users SELECT name, age)`
/// -> name:String vira nome:String, age:Int vira anos:Int.
///
/// Fica aqui, e não no type checker ou no interpretador, porque as DUAS fases
/// precisam renomear do mesmo jeito: o esquema que o type checker calcula para
/// uma CTE tem que ser exatamente o da tabela que o interpretador cria.
pub fn rename_columns(columns: &mut [Column], names: &[String]) -> Result<(), String> {
    if names.len() != columns.len() {
        return Err(format!(
            "declara {} coluna(s) ({}), mas o SELECT dela produz {}",
            names.len(),
            names.join(", "),
            columns.len(),
        ));
    }

    for (column, name) in columns.iter_mut().zip(names) {
        column.name = name.clone();
    }
    Ok(())
}

impl Table {
    /// Resolve uma referência de coluna (`coluna` ou `tabela.coluna`) para a
    /// posição dela. É a ÚNICA regra de resolução de nomes do projeto: o type
    /// checker e o interpretador chamam esta função, então uma query que passou
    /// na checagem acha exatamente as mesmas colunas na execução.
    ///
    /// - `tabela.coluna`: procura a coluna com esse nome vinda dessa tabela;
    /// - `coluna`: procura em todas as tabelas da query. Se o nome existir em
    ///   mais de uma, é AMBÍGUO e vira erro. Antes ele pegava a primeira em
    ///   silêncio: `ON name = name` comparava a coluna com ela mesma.
    pub fn resolve(&self, reference: &str) -> Result<usize, String> {
        let (source, name) = match reference.split_once('.') {
            Some((source, name)) => (Some(source), name),
            None => (None, reference),
        };

        let matches: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| {
                column.name == name
                    && source.map_or(true, |source| column.source.as_deref() == Some(source))
            })
            .map(|(index, _)| index)
            .collect();

        match (matches.as_slice(), source) {
            ([index], _) => Ok(*index),

            ([], Some(source)) if !self.columns.iter().any(|c| c.source.as_deref() == Some(source)) => Err(format!(
                "Tabela '{source}' não faz parte desta consulta (em '{reference}'); use o nome de uma tabela do FROM ou de um JOIN"
            )),

            ([], _) => Err(format!(
                "Coluna '{reference}' não existe (colunas disponíveis: {})",
                self.qualified_names().join(", ")
            )),

            (_, None) => {
                let sources: Vec<&str> = matches
                    .iter()
                    .filter_map(|&i| self.columns[i].source.as_deref())
                    .collect();
                let options: Vec<String> = sources.iter().map(|s| format!("{s}.{name}")).collect();
                Err(format!(
                    "Coluna '{name}' é ambígua: existe em {}; escreva {}",
                    sources.join(" e "),
                    options.join(" ou "),
                ))
            }

            // `tabela.coluna` com mais de um resultado: a mesma tabela aparece
            // duas vezes (`FROM users JOIN users`). Sem apelidos (AS), não há
            // como distinguir.
            (_, Some(source)) => Err(format!(
                "Coluna '{reference}' é ambígua: a tabela '{source}' aparece mais de uma vez na consulta"
            )),
        }
    }

    /// Nomes das colunas para mensagens de erro: `tabela.coluna` quando a
    /// origem é conhecida (dentro de uma query), senão só `coluna`.
    fn qualified_names(&self) -> Vec<String> {
        self.columns
            .iter()
            .map(|column| match &column.source {
                Some(source) => format!("{source}.{}", column.name),
                None => column.name.clone(),
            })
            .collect()
    }

    /// Posição de uma coluna pelo nome.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|column| column.name == name)
    }

    /// Coluna pelo nome.
    pub fn column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.name == name)
    }

    pub fn render(&self) -> Vec<String> {
        let width = |s: &str| s.chars().count();

        let mut widths: Vec<usize> = self.columns.iter().map(|c| width(&c.name)).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(width(cell));
            }
        }

        let format_row = |cells: Vec<&str>| -> String {
            cells
                .iter()
                .enumerate()
                .map(|(i, cell)| format!("{}{}", cell, " ".repeat(widths[i] - width(cell))))
                .collect::<Vec<_>>()
                .join(" | ")
                .trim_end()
                .to_string()
        };

        let mut lines = Vec::new();
        lines.push(format_row(self.columns.iter().map(|c| c.name.as_str()).collect()));
        lines.push(
            widths
                .iter()
                .map(|w| "-".repeat(*w))
                .collect::<Vec<_>>()
                .join("-+-"),
        );
        for row in &self.rows {
            lines.push(format_row(row.iter().map(String::as_str).collect()));
        }

        let count = self.rows.len();
        lines.push(format!("({} {})", count, if count == 1 { "linha" } else { "linhas" }));

        lines
    }
}