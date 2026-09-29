use crate::ast::*;
use crate::mini_sql_parser::*;

pub struct AstBuilder;

impl AstBuilder {
    pub fn build_program(ctx: &ProgramContext) -> Result<Program, String> {
        let mut queries = Vec::new();
        for q_ctx in ctx.query_children() {
            queries.push(Self::build_query(&q_ctx)?);
        }
        Ok(Program { queries })
    }

    pub fn build_query(ctx: &QueryContext) -> Result<Query, String> {
        let select_ctx = ctx
            .select_list()
            .map_err(|e| format!("Clausula SELECT invalida ou ausente: {:?}", e))?;

        let select = Self::build_select_list(&select_ctx)?;

        // Extrai o nome da tabela a partir do texto do contexto (após o FROM)
        let full_text = ctx.text();
        let table = if let Some(from_idx) = full_text.to_uppercase().find("FROM") {
            let after_from = &full_text[from_idx + 4..];
            after_from
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == ';' || c == ' ')
                .to_string()
        } else {
            full_text
        };

        let condition = if let Some(cond_ctx) = ctx.condition() {
            Some(Self::build_condition(&cond_ctx)?)
        } else {
            None
        };

        Ok(Query {
            table,
            select,
            condition,
        })
    }

    fn build_select_list(ctx: &SelectListContext) -> Result<SelectList, String> {
        let text = ctx.text();
        if text.trim() == "*" {
            Ok(SelectList::All)
        } else {
            let cols = text
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(SelectList::Columns(cols))
        }
    }

    fn build_condition(ctx: &ConditionContext) -> Result<Condition, String> {
        let children: Vec<_> = ctx.condition_children().collect();

        // Caso 1: NOT (Possui 1 sub-condição)
        if children.len() == 1 {
            let text = ctx.text().to_uppercase();
            if text.starts_with("NOT") {
                return Ok(Condition::Not(Box::new(Self::build_condition(&children[0])?)));
            }
        }

        // Caso 2: AND / OR (Possui 2 sub-condições)
        if children.len() >= 2 {
            let left = Self::build_condition(&children[0])?;
            let right = Self::build_condition(&children[1])?;

            let text = ctx.text().to_uppercase();
            if text.contains(" AND ") {
                return Ok(Condition::And(Box::new(left), Box::new(right)));
            } else if text.contains(" OR ") {
                return Ok(Condition::Or(Box::new(left), Box::new(right)));
            }
        }

        // Caso 3: Expressão folha (Comparação simples)
        if let Some(expr_ctx) = ctx.expr() {
            return Ok(Condition::Expr(Self::build_expr(&expr_ctx)?));
        }

        // Caso 4: Expressão IN
        if let Some(in_ctx) = ctx.expressao_in() {
            return Ok(Condition::ExprIn(Self::build_expr_in(&in_ctx)?));
        }

        Err("Tipo de condicao nao reconhecido".to_string())
    }

    fn build_expr(ctx: &ExprContext) -> Result<Expr, String> {
        // value_children() em vez de value_all()
        let values: Vec<_> = ctx.value_children().collect();
        if values.len() < 2 {
            return Err("Expressao de comparacao incompleta".to_string());
        }

        let left = Self::build_value(&values[0])?;
        let right = Self::build_value(&values[1])?;

        let text = ctx.text();
        let op = if text.contains(">=") {
            Operator::GreaterEqual
        } else if text.contains("<=") {
            Operator::LessEqual
        } else if text.contains("!=") || text.contains("<>") {
            Operator::NotEqual
        } else if text.contains('>') {
            Operator::Greater
        } else if text.contains('<') {
            Operator::Less
        } else if text.contains('=') {
            Operator::Equal
        } else {
            return Err(format!("Operador desconhecido: {}", text));
        };

        Ok(Expr { left, op, right })
    }

    fn build_expr_in(ctx: &ExpressaoInContext) -> Result<ExprIn, String> {
        // value_children() em vez de value_all()
        let values_ctx: Vec<_> = ctx.value_children().collect();
        if values_ctx.is_empty() {
            return Err("Expressao IN vazia".to_string());
        }

        let value = Self::build_value(&values_ctx[0])?;
        let mut values = Vec::new();

        for v_ctx in &values_ctx[1..] {
            values.push(Self::build_value(v_ctx)?);
        }

        Ok(ExprIn { value, values })
    }

    fn build_value(ctx: &ValueContext) -> Result<Value, String> {
        let text = ctx.text();

        if text.starts_with('\'') || text.starts_with('"') {
            let inner = &text[1..text.len() - 1];
            Ok(Value::String(inner.to_string()))
        } else if text.eq_ignore_ascii_case("true") {
            Ok(Value::Boolean(true))
        } else if text.eq_ignore_ascii_case("false") {
            Ok(Value::Boolean(false))
        } else if let Ok(v) = text.parse::<i64>() {
            Ok(Value::Int(v))
        } else if let Ok(v) = text.parse::<f64>() {
            Ok(Value::Float(v))
        } else {
            Ok(Value::Id(text))
        }
    }
}