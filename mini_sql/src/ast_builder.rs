use crate::ast::*;
use crate::mini_sql_lexer::MiniSqlLexer;
use crate::mini_sql_parser::{
    self, miniSQLVisitor, AndLabelContext, ConditionContext, ExprContext, ExprCondLabelContext,
    ExpressaoInContext, InCondLabelContext, MiniSqlParser, NotLabelContext, OrLabelContext,
    ParensLabelContext, ProgramContext, QueryContext, SelectListContext, ValueContext,
};

use antlr4_runtime::FromRuleNode;

type Res<T> = Result<T, String>; // facilita no tratamento de erros

fn miss<E>(_e: E) -> String {
    "nó filho ausente na árvore (possível erro de sintaxe)".to_string()
}

pub struct AstBuilder;//namespace - não guarda estado

impl AstBuilder { //metodos associados
    /// Ponto de entrada: recebe o texto SQL e devolve a AST.
    pub fn parse_sql(input: &str) -> Res<Program> {
        let parsed = mini_sql_parser::parse(input, MiniSqlLexer::new, MiniSqlParser::program)
            .map_err(|e| format!("{e:?}"))?;

        let rule = parsed
            .tree()
            .as_rule()
            .ok_or("a raiz da árvore não é uma regra")?;

        let root = ProgramContext::from_rule_node(rule)
            .ok_or("a raiz da árvore não é um `program`")?;

        Self::build_program(&root)
    }

    pub fn build_program(ctx: &ProgramContext<'_>) -> Res<Program> {
        let queries = ctx
            .query_children()
            .map(|q| Self::build_query(&q))
            .collect::<Res<Vec<_>>>()?;
        Ok(Program { queries })
    }

    fn build_query(ctx: &QueryContext<'_>) -> Res<Query> {
        // query : FROM ID SELECT selectList (WHERE condition)? END
        let table = ctx.id_token().map_err(miss)?.to_string();
        let select = Self::build_select_list(&ctx.select_list().map_err(miss)?);

        let condition = match ctx.condition() {
            Some(c) => Some(Self::build_condition(&c)?),
            None => None,
        };

        Ok(Query { table, select, condition })
    }

    fn build_select_list(ctx: &SelectListContext<'_>) -> SelectList {
        // Os tokens são concatenados sem espaço: "*" ou "a,b,c".
        let text = ctx.text().to_string();
        if text == "*" {
            SelectList::All
        } else {
            SelectList::Columns(
                text.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            )
        }
    }

    fn build_condition(ctx: &ConditionContext<'_>) -> Res<Condition> {
        ConditionBuilder.visit(ctx)
    }

    fn build_expr(ctx: &ExprContext<'_>) -> Res<Expr> {
        let left = Self::build_value(&ctx.left().map_err(miss)?)?;
        let right = Self::build_value(&ctx.right().map_err(miss)?)?;

        let op = if ctx.equal_token().is_some() {
            Operator::Equal
        } else if ctx.not_equal_token().is_some() {
            Operator::NotEqual
        } else if ctx.less_equal_token().is_some() {
            Operator::LessEqual
        } else if ctx.less_token().is_some() {
            Operator::Less
        } else if ctx.greater_equal_token().is_some() {
            Operator::GreaterEqual
        } else if ctx.greater_token().is_some() {
            Operator::Greater
        } else {
            return Err("operador de comparação não reconhecido".to_string());
        };

        Ok(Expr { left, op, right })
    }

    fn build_in(ctx: &ExpressaoInContext<'_>) -> Res<ExprIn> {
        // expressaoIn : value IN ( value (, value)* )
        // O primeiro `value` é o testado; os demais são a lista.
        let mut vals = ctx
            .value_children()
            .map(|v| Self::build_value(&v))
            .collect::<Res<Vec<_>>>()?;

        if vals.is_empty() {
            return Err("IN sem valores".to_string());
        }
        let value = vals.remove(0);
        Ok(ExprIn { value, values: vals })
    }

    fn build_value(ctx: &ValueContext<'_>) -> Res<Value> {
        let text = ctx.text().to_string();

        if ctx.id_token().is_some() {
            Ok(Value::Id(text))
        } else if ctx.int_token().is_some() {
            text.parse::<i64>().map(Value::Int).map_err(|e| e.to_string())
        } else if ctx.float_token().is_some() {
            text.parse::<f64>().map(Value::Float).map_err(|e| e.to_string())
        } else if ctx.string_token().is_some() {
            // o lexer não permite aspas dentro da string
            Ok(Value::String(text.trim_matches('\'').to_string()))
        } else if ctx.boolean_token().is_some() {
            Ok(Value::Boolean(text.eq_ignore_ascii_case("true")))
        } else {
            Err(format!("valor não reconhecido: {text}"))
        }
    }
}

/// Visitor do parser
// complexidade-> recursão , alternativas rotuladas
struct ConditionBuilder;

impl miniSQLVisitor for ConditionBuilder {
    type Result = Res<Condition>;

    fn default_result(&mut self) -> Self::Result {
        Err("condição não reconhecida".to_string())
    }

    fn visit_not_label(&mut self, ctx: &NotLabelContext) -> Self::Result {
        let inner = self.visit(ctx.inner().map_err(miss)?)?;
        Ok(Condition::Not(Box::new(inner)))
    }

    fn visit_and_label(&mut self, ctx: &AndLabelContext) -> Self::Result {
        let left = self.visit(ctx.left().map_err(miss)?)?;
        let right = self.visit(ctx.right().map_err(miss)?)?;
        Ok(Condition::And(Box::new(left), Box::new(right)))
    }

    fn visit_or_label(&mut self, ctx: &OrLabelContext) -> Self::Result {
        let left = self.visit(ctx.left().map_err(miss)?)?;
        let right = self.visit(ctx.right().map_err(miss)?)?;
        Ok(Condition::Or(Box::new(left), Box::new(right)))
    }

    fn visit_parens_label(&mut self, ctx: &ParensLabelContext) -> Self::Result {
        let inner = self.visit(ctx.inner().map_err(miss)?)?;
        Ok(Condition::Parens(Box::new(inner)))
    }

    fn visit_expr_cond_label(&mut self, ctx: &ExprCondLabelContext) -> Self::Result {
        let expr = AstBuilder::build_expr(&ctx.expr().map_err(miss)?)?;
        Ok(Condition::Expr(expr))
    }

    fn visit_in_cond_label(&mut self, ctx: &InCondLabelContext) -> Self::Result {
        let e = AstBuilder::build_in(&ctx.expressao_in().map_err(miss)?)?;
        Ok(Condition::ExprIn(e))
    }
}