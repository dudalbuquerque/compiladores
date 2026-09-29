#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub queries: Vec<Query>,
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


