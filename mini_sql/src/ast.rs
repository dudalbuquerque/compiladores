#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Query(Query),
    Load(Load),
    Describe(Describe),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Load {
    pub file: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Describe {
    pub table: String,
}

/// `WITH a AS (...), b AS (...) FROM ... ;`
/// As CTEs vêm antes do corpo e só existem dentro desta query.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub ctes: Vec<Cte>,
    pub body: QueryBody,
}

/// `nome AS (FROM ...)` ou `nome (c1, c2) AS (FROM ...)`.
/// `columns` é `None` quando não há lista: as colunas vêm do SELECT do corpo.
#[derive(Debug, Clone, PartialEq)]
pub struct Cte {
    pub name: String,
    pub columns: Option<Vec<String>>,
    pub body: QueryBody,
}

/// `FROM ... SELECT ... WHERE ...` sem o `;`.
/// É o corpo da query principal e também o corpo de cada CTE.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryBody {
    pub table: String,
    pub joins: Vec<Join>,
    pub select: SelectList,
    pub condition: Option<Condition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    pub join_type: JoinType,
    pub table: String,
    pub condition: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectList {
    Columns(Vec<String>),
    All,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Parens(Box<Condition>),
    Expr(Expr),
    ExprIn(ExprIn),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub left: Value,
    pub op: Operator,
    pub right: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprIn {
    pub value: Value,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Id(String),
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}


// Formatação legível dos nós, usada nas mensagens de erro
// (ex.: "idade >= 'vinte'" em vez de "Expr { left: Id(\"idade\"), ... }").

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Id(name) => write!(f, "{name}"),
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::String(v) => write!(f, "'{v}'"),
            Value::Boolean(v) => write!(f, "{}", if *v { "TRUE" } else { "FALSE" }),
        }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbol = match self {
            Operator::Equal => "=",
            Operator::NotEqual => "!=",
            Operator::Less => "<",
            Operator::LessEqual => "<=",
            Operator::Greater => ">",
            Operator::GreaterEqual => ">=",
        };
        write!(f, "{symbol}")
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.left, self.op, self.right)
    }
}