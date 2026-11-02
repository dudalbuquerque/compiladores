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

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub table: String,
    pub select: SelectList,
    pub condition: Option<Condition>,
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