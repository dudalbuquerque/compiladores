use crate::ast::*;
use crate::mini_sql_lexer::MiniSqlLexer;
use crate::mini_sql_parser::{
    self, miniSQLVisitor, AndLabelContext, ConditionContext, CteContext, ExprContext, ExprCondLabelContext,
    ExpressaoInContext, InCondLabelContext, JoinContext, MiniSqlParser, NotLabelContext, OrLabelContext,
    ParensLabelContext, ProgramContext, QueryBodyContext, QueryContext, SelectListContext, StatementContext,
    ValueContext,
};

use std::sync::{Arc, Mutex};

use antlr4_runtime::parser::Parser;
use antlr4_runtime::{ErrorListener, FromRuleNode, Recognizer, SyntaxErrorEvent};

type Res<T> = Result<T, String>; // facilita no tratamento de erros

/// Listener de erros que GUARDA as mensagens em vez de imprimi-las.
///
/// O runtime exige que o listener seja `Send + 'static` e o recebe por valor,
/// então lexer e parser ficam cada um com um clone; o `Arc<Mutex<..>>` faz os
/// clones escreverem na mesma lista, que o `parse_sql` lê no final.
#[derive(Clone, Default)]
struct SyntaxErrors(Arc<Mutex<Vec<String>>>);

impl SyntaxErrors {
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl<R: Recognizer + ?Sized> ErrorListener<R> for SyntaxErrors {
    fn syntax_error(&mut self, _recognizer: &R, event: &SyntaxErrorEvent<'_>) {
        // O ANTLR conta a coluna a partir de 0; para quem lê, a partir de 1.
        self.0.lock().unwrap().push(format!(
            "linha {}, coluna {}: {}",
            event.line,
            event.column + 1,
            event.message
        ));
    }
}

fn miss<E>(_e: E) -> String {
    "nó filho ausente na árvore (possível erro de sintaxe)".to_string()
}

pub struct AstBuilder;//namespace - não guarda estado

impl AstBuilder { //metodos associados
    /// Ponto de entrada: recebe o texto SQL e devolve a AST.
    ///
    /// Se houver QUALQUER erro léxico ou sintático, devolve `Err` com as
    /// mensagens (linha e coluna) e nenhuma AST.
    ///
    /// Isso precisa ser feito à mão porque o parser do ANTLR se RECUPERA de
    /// erros: ele insere o token que faltou ou descarta o que sobrou e segue
    /// em frente, devolvendo uma árvore "consertada". Antes, essa árvore virava
    /// AST e era executada:
    ///   - `FROM users SELECT name`  (sem `;`)       -> executava normalmente;
    ///   - `... WHERE age >= ;`      (falta o valor) -> o ANTLR inventava um valor.
    /// A recuperação é útil para editores (mostrar vários erros de uma vez),
    /// mas um interpretador não deve executar um programa que não é da linguagem.
    pub fn parse_sql(input: &str) -> Res<Program> {
        // Coletor compartilhado pelo lexer e pelo parser.
        let errors = SyntaxErrors::default();

        let output = mini_sql_parser::parse_with_parser_constructor(
            input,
            |chars| {
                let mut lexer = MiniSqlLexer::new(chars);
                // Troca o listener padrão (que só imprime no stderr) pelo coletor.
                lexer.remove_error_listeners();
                lexer.add_error_listener(errors.clone());
                lexer
            },
            |tokens| {
                let mut parser = MiniSqlParser::new(tokens);
                parser.remove_error_listeners();
                parser.add_error_listener(errors.clone());
                parser
            },
            MiniSqlParser::program,
        )
        .map_err(|e| format!("{e:?}"))?;

        let messages = errors.take();
        if !messages.is_empty() {
            return Err(messages.join("; "));
        }
        // Proteção: o parser contou um erro que nenhum listener recebeu.
        if output.parser.number_of_syntax_errors() > 0 {
            return Err("entrada inválida".to_string());
        }

        let parsed = output.into_parsed_file();

        let rule = parsed
            .tree()
            .as_rule()
            .ok_or("a raiz da árvore não é uma regra")?;

        let root = ProgramContext::from_rule_node(rule)
            .ok_or("a raiz da árvore não é um `program`")?;

        Self::build_program(&root)
    }

    // adiciona mais statements 
    pub fn build_program(ctx: &ProgramContext<'_>) -> Res<Program> {
        let statements = ctx
            .statement_children()
            .map(|s| Self::build_statement(&s))
            .collect::<Res<Vec<_>>>()?;

        Ok(Program { statements })
    }

    fn build_statement(ctx: &StatementContext<'_>) -> Res<Statement> {
        if let Some(load) = ctx.load() {
            let file = load.string_token().map_err(miss)?.to_string();
            let file = file.trim_matches(|c| c == '\'' || c == '"').to_string();

            Ok(Statement::Load(Load { file }))
        } else if let Some(describe) = ctx.describe() {
            let table = describe.id_token().map_err(miss)?.to_string();

            Ok(Statement::Describe(Describe { table }))
        } else if let Some(query) = ctx.query() {
            Ok(Statement::Query(Self::build_query(&query)?))
        } else {
            Err("tipo de statement não reconhecido".to_string())
        }
    }

    fn build_query(ctx: &QueryContext<'_>) -> Res<Query> {
        // query : cteClause? queryBody END
        // cteClause : WITH cte (COMMA cte)*
        let ctes = match ctx.cte_clause() {
            Some(clause) => clause
                .cte_children()
                .map(|c| Self::build_cte(&c))
                .collect::<Res<Vec<_>>>()?,
            None => Vec::new(),
        };

        let body = Self::build_query_body(&ctx.query_body().map_err(miss)?)?;

        Ok(Query { ctes, body })
    }

    fn build_cte(ctx: &CteContext<'_>) -> Res<Cte> {
        // cte : ID (LPAREN ID (COMMA ID)* RPAREN)? AS LPAREN queryBody RPAREN
        // O gerador junta todos os IDs da regra numa lista só:
        // o primeiro é o nome da CTE; os demais, se existirem, são as colunas.
        let mut ids = ctx.id_tokens().map(|id| id.to_string());

        let name = ids.next().ok_or_else(|| miss(()))?;

        let names: Vec<String> = ids.collect();
        // A gramática exige pelo menos um ID dentro dos parênteses,
        // então lista vazia significa "sem lista de colunas".
        let columns = if names.is_empty() { None } else { Some(names) };

        let body = Self::build_query_body(&ctx.query_body().map_err(miss)?)?;

        Ok(Cte { name, columns, body })
    }

    fn build_query_body(ctx: &QueryBodyContext<'_>) -> Res<QueryBody> {
        // queryBody : FROM ID (join)* SELECT selectList (WHERE condition)?
        let table = ctx.id_token().map_err(miss)?.to_string();

        let joins = ctx
            .join_children()
            .map(|j| Self::build_join(&j))
            .collect::<Res<Vec<_>>>()?;

        let select = Self::build_select_list(&ctx.select_list().map_err(miss)?);

        let condition = match ctx.condition() {
            Some(c) => Some(Self::build_condition(&c)?),
            None => None,
        };

        Ok(QueryBody { table, joins, select, condition })
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

    fn build_join(ctx: &JoinContext<'_>) -> Res<Join> {
        // O texto do nó vem com os tokens colados ("LEFTOUTERJOINpedidos..."),
        // então basta olhar a primeira palavra-chave. Sem tipo = INNER.
        let text = ctx.text().to_uppercase();
        let join_type = if text.starts_with("LEFT") {
            JoinType::Left
        } else if text.starts_with("RIGHT") {
            JoinType::Right
        } else if text.starts_with("FULL") {
            JoinType::Full
        } else {
            JoinType::Inner
        };

        // Desembrulha o Option<&str> retornado por text() com .expect()
        let table = ctx.id_token().map_err(miss)?.symbol().text().expect("esperado texto no token ID").to_string();
        
        let condition = Self::build_expr(&ctx.expr().map_err(miss)?)?;

        Ok(Join { 
            join_type, 
            table, 
            condition 
        })
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

        if ctx.column_ref().is_some() {
            // `coluna` ou `tabela.coluna`. Guardamos o texto como foi escrito
            // (os tokens vêm concatenados, sem espaço: "pedidos.id"); quem
            // resolve a qual coluna ele se refere é `Table::resolve`.
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