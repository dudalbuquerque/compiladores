pub struct Query {
    pub table: String,
    pub select: SelectList,
    pub condition: Option<Condition>,
}

pub enum SelectList {
    Columns(Vec<String>),
    All,
}

pub enum Condition {
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Parens(Box<Condition>),
    Expr(Expr),
    ExprIn(ExprIn),
}

pub struct Expr {
    pub left: Value,
    pub op: Operator,
    pub right: Value,
}

pub struct ExprIn {
    pub value: Value,
    pub values: Vec<Value>,
}

pub enum Value {
    Id(String),
    Int(i32),
    Float(f64),
    String(String),
    Boolean(bool),
}

pub enum Operator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}


