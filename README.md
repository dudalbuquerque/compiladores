# MiniSQL

Implementação de uma linguagem de consulta baseada em um subconjunto de SQL, desenvolvida para a disciplina de **Compiladores**. A gramática é escrita em ANTLR4 e o analisador é gerado em **Rust**.

Este documento descreve o estado atual do projeto: da definição da gramática até a execução, incluindo **JOIN**, **CTEs** (`WITH`) e **nomes qualificados** (`tabela.coluna`).

---

## 1. Estado atual

| Etapa | Situação |
|---|---|
| Definição da gramática (`miniSQL.g4`) | concluída |
| Análise léxica | concluída (lexer gerado) |
| Análise sintática | concluída (parser gerado); entradas com erro de sintaxe são **rejeitadas** |
| Construção da AST | concluída (`ast_builder.rs`) |
| Análise semântica | concluída (`type_checker.rs`) |
| Execução | concluída (`interpreter.rs`) |
| `INNER JOIN` / `LEFT JOIN` | concluído |
| CTEs (`WITH ... AS (...)`) | concluído, com modo `.trace` para visualizar as tabelas temporárias |
| Nomes qualificados (`tabela.coluna`) | concluído, com detecção de nome ambíguo |
| Suíte de testes | 207 testes, todos passando |

---

## 2. A linguagem

A MiniSQL adota deliberadamente a **ordem invertida** em relação ao SQL padrão: a tabela é declarada antes das colunas.

```sql
FROM users SELECT name, credit WHERE credit >= 150.50 AND active = TRUE;
```

A motivação é que o leitor (e o analisador) conhece a origem dos dados antes de encontrar os nomes de coluna. Como consequência, uma consulta SQL convencional é **rejeitada** pela linguagem — comportamento verificado pelo teste `test_old_select_syntax`.

### O que é aceito

| Construção | Exemplo |
|---|---|
| Carga de dados | `LOAD 'test_data/users.csv';` (cria a tabela `users`) |
| Esquema de uma tabela | `DESCRIBE users;` ou `DESC users;` |
| Projeção de todas as colunas | `FROM users SELECT *;` |
| Projeção de colunas | `FROM users SELECT name, age;` |
| Comparações | `=` `!=` `<>` `<` `<=` `>` `>=` |
| Conectivos lógicos | `NOT`, `AND`, `OR` |
| Agrupamento | `WHERE NOT (age < 18 OR active = FALSE);` |
| Pertinência | `WHERE type IN ('admin', 'editor');` |
| Junção | `FROM usuarios JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT nome, produto;` |
| Junção externa à esquerda | `FROM usuarios LEFT JOIN pedidos ON ...` (sem correspondência vira NULL) |
| Junções encadeadas | `FROM a JOIN b ON ... JOIN c ON ...` |
| Nome qualificado | `usuarios.id`, no `SELECT`, no `WHERE` e no `ON` |
| CTE | `WITH adultos AS (FROM users SELECT name WHERE age >= 18) FROM adultos SELECT name;` |
| CTE com lista de colunas | `WITH adultos (nome, anos) AS (FROM users SELECT name, age) ...` |
| Várias CTEs | `WITH a AS (...), b AS (FROM a ...) FROM b ...` |
| Literais | inteiro, decimal, negativo, string, booleano |
| Múltiplas instruções | separadas por `;` no mesmo arquivo |
| Comentários | `-- até o fim da linha` |
| Palavras-chave | `SELECT` ou `select` (caixa uniforme) |

### O que não é aceito

`ORDER BY`, `GROUP BY`, `LIMIT`, `DISTINCT`, funções de agregação, subconsultas, expressões aritméticas, `IS NULL`, `UNION`, CTEs recursivas (`WITH RECURSIVE`) e apelidos de tabela (`FROM users AS u`). A condição do `ON` é uma única comparação (sem `AND`). Identificadores devem começar com letra minúscula (`[a-z] [a-z0-9_]*`).

---

## 3. Arquitetura

O projeto tem dois momentos distintos: a **geração do analisador**, que acontece durante o desenvolvimento, e o **processamento de uma consulta**, que acontece em execução.

### 3.1 Geração do analisador (tempo de desenvolvimento)

```
   grammar/miniSQL.g4          ← ÚNICO artefato escrito à mão nesta etapa
   (regras de lexer + parser)
            │
            │   antlr4-rust-gen --visitor --out-dir generated grammar/miniSQL.g4
            ▼
   generated/
     ├── mini_sql_lexer.rs     ← analisador léxico gerado
     ├── mini_sql_parser.rs    ← analisador sintático + visitor gerados
     ├── decisions.json        ← relatório das decisões do parser
     └── semantics.json        ← relatório de predicados semânticos
```

Os arquivos de `generated/` são versionados no repositório para que o projeto compile sem exigir a instalação do gerador. Eles **não devem ser editados à mão**: qualquer alteração se perde na próxima geração. A fonte da verdade é o `.g4`. A opção `--visitor` é obrigatória: o `ast_builder` usa o visitor gerado.

### 3.2 Processamento de uma consulta (tempo de execução)

![Pipeline de processamento de uma consulta MiniSQL: análise léxica produz tokens, análise sintática produz a parse tree, o AstBuilder produz a AST, o TypeChecker verifica a AST contra o esquema das tabelas carregadas e o interpretador executa a consulta e produz a saída](docs/arquitetura-pipeline.png)

As fases 1 e 2 são **código gerado** pelo ANTLR a partir da gramática. As fases 3, 4 e 5 são **escritas à mão**. O fonte do diagrama está em [`docs/arquitetura-pipeline.html`](docs/arquitetura-pipeline.html).

Um passo intermediário que o diagrama resume: entre a análise léxica e a sintática existe um fluxo de tokens. Para a consulta `FROM users SELECT name WHERE credit >= 150.50;`, o parser recebe exatamente isto:

```text
[FROM] [ID:users] [SELECT] [ID:name] [WHERE] [ID:credit] [GREATER_EQUAL] [FLOAT:150.50] [END:;] [EOF]
```

O pipeline (`src/pipeline.rs`) trabalha em duas granularidades:

- **As fases 1–3 rodam sobre o texto inteiro.** Se houver qualquer erro de sintaxe, nada é executado — nem as instruções anteriores ao erro.
- **As fases 4 e 5 rodam uma instrução por vez.** O esquema de uma tabela só existe depois que o `LOAD` dela executa, então o type checker só consegue verificar `FROM users ...` depois do `LOAD 'users.csv';` anterior.

O pipeline para no primeiro erro e o prefixa com a fase que o detectou: `Erro de sintaxe`, `Erro semântico` ou `Erro de execução`.

### 3.3 O que é gerado e o que é nosso

Esta é a divisão mais importante do projeto: **o parser não foi escrito por nós**. O que escrevemos foi a *definição da linguagem*; o algoritmo que reconhece essa linguagem é derivado dela automaticamente. A análise semântica e a execução, por outro lado, são inteiramente nossas.

| Componente | Linhas | Origem |
|---|---|---|
| `grammar/miniSQL.g4` | 137 | escrito à mão |
| `generated/mini_sql_lexer.rs` | 158 | gerado pelo ANTLR |
| `generated/mini_sql_parser.rs` | 3072 | gerado pelo ANTLR |
| `src/ast.rs` | 143 | escrito à mão |
| `src/ast_builder.rs` | 329 | escrito à mão |
| `src/type_checker.rs` | 320 | escrito à mão |
| `src/interpreter.rs` | 403 | escrito à mão |
| `src/table.rs` | 219 | escrito à mão |
| `src/csv_loader.rs` | 111 | escrito à mão |
| `src/pipeline.rs` | 45 | escrito à mão |
| `tests/` | 2086 | escrito à mão |

Consequência prática para a suíte de testes: quando um teste das fases 1–2 falha, o defeito está quase sempre na **gramática** ou na **expectativa do teste** — o ANTLR é infraestrutura de terceiros, com a própria suíte de validação.

---

## 4. Estrutura de arquivos

```text
mini_sql/
├── generated/              código gerado pelo ANTLR (não editar)
├── grammar/
│   └── miniSQL.g4
├── src/
│   ├── ast.rs
│   ├── ast_builder.rs
│   ├── ast_printer.rs
│   ├── csv_loader.rs
│   ├── interpreter.rs
│   ├── lib.rs
│   ├── main.rs
│   ├── pipeline.rs
│   ├── table.rs
│   └── type_checker.rs
├── test_data/
│   ├── alunos.csv
│   ├── disciplinas.csv
│   ├── pedidos.csv
│   ├── users.csv
│   └── usuarios.csv
├── tests/
│   ├── ast_test.rs
│   ├── csv_loader_test.rs
│   ├── interpreter_test.rs
│   ├── parser_error_test.rs
│   ├── parser_success_test.rs
│   ├── pipeline_test.rs
│   └── type_checker_test.rs
├── Cargo.toml
└── Cargo.lock
```

| Arquivo | Responsabilidade |
|---|---|
| `lib.rs` | Expõe os módulos e integra o código gerado pelo ANTLR. |
| `main.rs` | Terminal interativo: lê comandos, chama o pipeline, mostra o resultado; comando `.trace`. |
| `ast.rs` | Define as estruturas da AST. |
| `ast_builder.rs` | Roda o parser, rejeita entradas com erro de sintaxe e constrói a AST. |
| `ast_printer.rs` | Exibe a AST para facilitar a visualização e depuração. |
| `type_checker.rs` | Análise semântica: tabelas, colunas, tipos, CTEs e nomes ambíguos. |
| `interpreter.rs` | Executa `LOAD`, `DESCRIBE` e consultas (JOIN, CTEs, WHERE, projeção). |
| `pipeline.rs` | Encadeia parser → type checker → interpretador e prefixa os erros com a fase. |
| `table.rs` | Banco de dados, tabelas, colunas e tipos; regras compartilhadas pelas duas fases (`resolve`, `rename_columns`). |
| `csv_loader.rs` | Lê os CSVs e infere o tipo de cada coluna. |

---

## 5. A gramática

Arquivo: [`mini_sql/grammar/miniSQL.g4`](mini_sql/grammar/miniSQL.g4)

### Regras de parser

```antlr
program     : statement+ EOF;
statement   : query
            | load
            | describe;
load        : LOAD STRING END;
describe    : DESCRIBE ID END ;
query       : cteClause? queryBody END ;
cteClause   : WITH cte (COMMA cte)* ;
cte         : ID (LPAREN ID (COMMA ID)* RPAREN)? AS LPAREN queryBody RPAREN ;
queryBody   : FROM ID (join)* SELECT selectList (WHERE condition)? ;
join        : (INNER | LEFT)? JOIN ID ON expr ;
selectList  : columnRef (COMMA columnRef)* | STAR ;
columnRef   : ID (DOT ID)? ;
condition   : NOT inner=condition                  # not
            | left=condition AND right=condition   # and
            | left=condition OR  right=condition   # or
            | LPAREN inner=condition RPAREN        # parens
            | expr                                 # exprCond
            | expressaoIn                          # inCond
            ;
expr        : left=value op=(EQUAL|NOT_EQUAL|LESS|LESS_EQUAL|GREATER|GREATER_EQUAL) right=value ;
expressaoIn : value IN LPAREN value (COMMA value)* RPAREN ;
value       : columnRef | INT | FLOAT | STRING | BOOLEAN ;
```

### Decisões de projeto

**Precedência por ordem das alternativas.** A regra `condition` é recursiva à esquerda, e o ANTLR resolve a precedência pela ordem em que as alternativas aparecem: a primeira liga mais forte. Daí `NOT` > `AND` > `OR`, que é a precedência do SQL padrão. Comportamento verificado na prática:

| Entrada | AST resultante |
|---|---|
| `a = 1 OR b = 2 AND c = 3` | `Or(a, And(b, c))` — `AND` liga mais forte |
| `a = 1 AND b = 2 AND c = 3` | `And(And(a, b), c)` — associativo à esquerda |
| `NOT a = 1 AND b = 2` | `And(Not(a), b)` — `NOT` liga mais forte |

**Rótulos de alternativa (`# not`, `# and`, …).** Sem eles, cada nó de condição chegaria ao código Rust como um `ConditionContext` genérico, e descobrir qual alternativa foi usada exigiria procurar tokens dentro do nó. Com os rótulos, o ANTLR gera um tipo distinto por alternativa (`AndLabelContext`, `OrLabelContext`, …) e o `ast_builder` faz correspondência direta.

**`queryBody` separado de `query`.** O corpo `FROM ... SELECT ... WHERE ...` (sem o `;`) é uma regra própria porque aparece em dois lugares: no corpo principal e dentro de cada CTE. A AST espelha isso com o tipo `QueryBody`, e o mesmo código verifica e executa os dois.

**`WITH RECURSIVE` foi retirado.** Uma CTE recursiva precisa de `UNION` (parte âncora `UNION ALL` parte recursiva), que a linguagem não tem. Aceitar a palavra sem efeito seria pior do que recusá-la.

**`columnRef : ID (DOT ID)?`.** Quando duas tabelas de uma consulta têm uma coluna com o mesmo nome (`usuarios.id` e `pedidos.id`), o prefixo é a única forma de dizer qual delas se quer. O token `DOT` não conflita com `FLOAT`: em `150.50`, o lexer escolhe o casamento mais longo, que é o `FLOAT`.

**`ERROR_CHARACTER : . ;` — tratamento de caractere inválido.** Esta é a última regra do lexer e casa qualquer caractere isolado. Sem ela, um caractere que não pertence a nenhum token — por exemplo o `_` inicial em `FROM _users` — é **descartado silenciosamente** pelo lexer: o parser recebe `FROM users SELECT name;`, uma consulta válida, e a entrada errada produz a **tabela errada sem qualquer erro**. Com a regra, o caractere vira um token que nenhuma regra de parser aceita, e o erro passa a ser reportado com linha e coluna:

```
Erro de sintaxe: linha 1, coluna 6: extraneous input '_' expecting ID
```

A posição no fim do arquivo é obrigatória: o lexer do ANTLR prefere o casamento mais longo e, em empate, a regra declarada primeiro. Como `.` casa exatamente um caractere, ele só vence quando nenhuma outra regra casou.

**Caixa das palavras-chave.** Cada palavra-chave lista explicitamente as duas formas (`SELECT : 'SELECT' | 'select';`). A opção global `caseInsensitive` foi descartada porque ela também tornaria `[a-z]` sensível a maiúsculas, fazendo `Users` ser aceito como identificador válido. O custo dessa escolha é que formas mistas como `Select` não são aceitas.

### Erros de sintaxe não são executados

O parser do ANTLR **se recupera** de erros: diante de uma entrada malformada, ele insere o token que faltou ou descarta o que sobrou, e devolve uma árvore "consertada" com `Ok`. Isso é útil em editores, que querem mostrar vários erros de uma vez, mas um interpretador não deve executar um programa que não pertence à linguagem. Antes da correção:

| Entrada | Comportamento antigo |
|---|---|
| `FROM users SELECT name` (sem `;`) | o ANTLR inseria o `;` e a consulta **executava** |
| `FROM users SELECT name WHERE age >= ;` | o ANTLR **inventava um valor**, e o erro aparecia como semântico (`... o literal FALSE é Boolean`) |

O `parse_sql` agora substitui o listener padrão do ANTLR (que só imprime no stderr) por um que **coleta** as mensagens, registrado no lexer e no parser. Se houver qualquer mensagem, devolve `Err` e nenhuma AST é construída:

```
Erro de sintaxe: linha 1, coluna 23: missing ';' at '<EOF>'
```

---

## 6. A AST

Arquivo: [`mini_sql/src/ast.rs`](mini_sql/src/ast.rs)

```rust
pub struct Program   { pub statements: Vec<Statement> }
pub enum   Statement { Query(Query), Load(Load), Describe(Describe) }
pub struct Load      { pub file: String }
pub struct Describe  { pub table: String }

pub struct Query     { pub ctes: Vec<Cte>, pub body: QueryBody }
pub struct Cte       { pub name: String, pub columns: Option<Vec<String>>, pub body: QueryBody }
pub struct QueryBody {
    pub table:     String,
    pub joins:     Vec<Join>,
    pub select:    SelectList,
    pub condition: Option<Condition>,
}

pub struct Join      { pub join_type: JoinType, pub table: String, pub condition: Expr }
pub enum   JoinType  { Inner, Left }

pub enum SelectList { Columns(Vec<String>), All }

pub enum Condition {
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Parens(Box<Condition>),
    Expr(Expr),
    ExprIn(ExprIn),
}

pub struct Expr   { pub left: Value, pub op: Operator, pub right: Value }
pub struct ExprIn { pub value: Value, pub values: Vec<Value> }

pub enum Value    { Id(String), Int(i64), Float(f64), String(String), Boolean(bool) }
pub enum Operator { Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual }
```

Observações:

- **`Cte.columns` é `Option`**, não uma lista que pode ser vazia, para distinguir "sem lista de colunas" de uma lista. A gramática já impede `()` vazio.
- **`Value::Id` guarda o nome como foi escrito**, inclusive com prefixo (`"pedidos.id"`). Quem decide a qual coluna ele se refere é a análise semântica (seção 7.2).

A construção é feita em [`ast_builder.rs`](mini_sql/src/ast_builder.rs), que usa o **visitor** gerado pelo ANTLR para a regra `condition` — cada alternativa rotulada vira um método (`visit_and_label`, `visit_or_label`, …) — e descida recursiva direta sobre os contextos tipados para as demais regras.

### Exemplo real

Saída produzida pelo teste `test_ast_generation` (resumida) para a consulta

```sql
FROM users SELECT name, credit
WHERE type IN ('admin', 'editor') AND active = TRUE AND credit >= 150.50;
```

```text
Program {
    statements: [
        Query(Query {
            ctes: [],
            body: QueryBody {
                table: "users",
                joins: [],
                select: Columns(["name", "credit"]),
                condition: Some(
                    And(
                        And(
                            ExprIn(ExprIn {
                                value: Id("type"),
                                values: [String("admin"), String("editor")],
                            }),
                            Expr(Expr { left: Id("active"), op: Equal, right: Boolean(true) }),
                        ),
                        Expr(Expr { left: Id("credit"), op: GreaterEqual, right: Float(150.5) }),
                    ),
                ),
            },
        }),
    ],
}
```

Repare no aninhamento `And(And(...), ...)`: é a associatividade à esquerda dos conectivos aparecendo na estrutura da árvore.

---

## 7. Análise semântica e execução

### 7.1 O type checker

Roda sobre a AST, **antes** da execução, olhando apenas o **esquema** das tabelas (nomes e tipos das colunas), nunca os dados. Rejeita:

- tabela inexistente (`FROM xyz` sem `LOAD` antes);
- coluna inexistente no `SELECT`, no `WHERE` ou no `ON`;
- comparação entre tipos incompatíveis (`age = 'vinte'`); `Int` e `Float` são compatíveis entre si;
- operador de ordem em tipo sem ordem (`active > TRUE`; `Boolean` só aceita `=` e `!=`);
- lista do `IN` com tipos misturados (`age IN (1, 'a')`);
- `LOAD` de arquivo que não é `.csv` ou cujo nome não vira um identificador (`alunos-2024.csv`);
- nome de coluna **ambíguo** (seção 7.2);
- CTE com nome repetido, com lista de colunas de tamanho errado ou com colunas repetidas (seção 7.3).

Os tipos das colunas são inferidos pelo `csv_loader` no `LOAD`: uma coluna é `Int` se todos os valores não vazios forem inteiros; senão `Float` se forem números; senão `Boolean` se forem `true`/`false`; senão `String`.

A função central é `check_query_body`, que **verifica um corpo de consulta e devolve o esquema do resultado** — as colunas que ele produz, com seus tipos. É a ideia clássica de type checker (verificar uma expressão e devolver o seu tipo), aplicada a consultas: o "tipo" de uma consulta é a lista de colunas da saída. É isso que permite usar uma CTE como tabela sem executá-la.

### 7.2 Resolução de nomes

Ao montar o escopo de uma consulta, cada coluna é marcada com a tabela de onde veio (o nome usado no `FROM` ou no `JOIN`). Uma única função, `Table::resolve`, decide a que coluna um nome se refere:

| Referência | Regra | Exemplo de erro |
|---|---|---|
| `tabela.coluna` | a coluna com esse nome vinda dessa tabela | `Tabela 'clientes' não faz parte desta consulta` |
| `coluna` | procura em todas as tabelas da consulta | `Coluna 'id' é ambígua: existe em usuarios e pedidos; escreva usuarios.id ou pedidos.id` |

Uma coluna que só existe numa tabela dispensa o prefixo (`usuario_id` em `usuarios JOIN pedidos`). Antes dessa regra, um nome repetido pegava a primeira coluna em silêncio: `ON name = name` comparava a coluna com ela mesma e devolvia um produto cartesiano.

`resolve` fica em `table.rs` e é chamada **pelas duas fases**: uma consulta aceita pelo type checker encontra exatamente as mesmas colunas na execução.

### 7.3 CTEs: tabelas temporárias com escopo

Uma CTE é uma tabela temporária que só existe dentro da consulta que a declarou. As duas fases usam a mesma estrutura de ambiente (`Env`), com duas camadas:

```text
busca de uma tabela:  1. CTEs desta consulta   →   2. tabelas carregadas (Database)
```

| | type checker | interpretador |
|---|---|---|
| a CTE guarda | só o esquema | as linhas |
| a CTE nasce | depois de verificada | depois de executada |
| a CTE morre | ao fim de `check_query` | ao fim de `run_query` |

Como cada CTE só entra no ambiente **depois** de processada, quatro regras do SQL saem dessa ordem, sem código específico:

- uma CTE enxerga as anteriores, nunca as seguintes;
- `WITH a AS (FROM a ...)` se refere à tabela `a` carregada, não a si mesma;
- uma CTE com o nome de uma tabela carregada a **esconde** dentro da consulta, como uma variável local esconde uma global;
- **a CTE some ao fim da consulta**: o `Env` é uma variável local, e o `Database` é recebido como `&` (somente leitura). O próprio compilador do Rust impede que uma CTE seja registrada no banco.

A lista de colunas `nome (c1, c2) AS (...)` renomeia **posição a posição**, mantendo os tipos. A função (`rename_columns`, em `table.rs`) também é compartilhada pelas duas fases. Uma CTE que produziria duas colunas com o mesmo nome (`SELECT *` de um JOIN em que as duas tabelas têm `id`) é rejeitada, e a mensagem indica a lista de colunas como solução.

### 7.4 O interpretador

- **Consulta = seleção + projeção.** Os JOINs são aplicados em sequência (o resultado de um vira a tabela da esquerda do próximo); depois, o `WHERE` filtra as linhas e o `SELECT` escolhe as colunas.
- **JOIN por laços aninhados** (*nested loop join*): cada linha da esquerda é comparada com cada linha da direita, O(n × m). No `LEFT JOIN`, uma linha da esquerda sem correspondência entra com as colunas da direita vazias.
- **NULL e lógica de três valores.** Uma célula vazia (no CSV ou vinda de um `LEFT JOIN`) vira NULL. Uma comparação com NULL não é verdadeira nem falsa: é *desconhecida* (`Option<bool>` = `None`). `NULL AND falso = falso`; `NULL OR verdadeiro = verdadeiro`; `NOT desconhecido = desconhecido`. Uma linha só entra no resultado se a condição der `Some(true)` — por isso a linha com `credit` vazio não aparece nem em `credit >= 150` nem em `credit < 150`, como no SQL.
- **Modo `.trace`.** No terminal, `.trace` liga ou desliga a exibição das tabelas temporárias: cada CTE aparece quando é criada (com o conteúdo) e quando é descartada. Consultas sem `WITH` não criam tabelas temporárias e não mostram nada a mais.

---

## 8. Testes

207 testes, distribuídos em sete arquivos. Cada arquivo vira um executável independente.

| Arquivo | Testes | Fase | O que valida |
|---|---|---|---|
| `parser_success_test.rs` | 40 | sintática | consultas válidas (incluindo `WITH` e `tabela.coluna`) são aceitas sem nenhum erro |
| `parser_error_test.rs` | 44 | sintática | consultas inválidas são rejeitadas |
| `ast_test.rs` | 9 | construção da AST | estrutura da AST, CTEs, nomes qualificados; AST não é gerada com erro de sintaxe |
| `csv_loader_test.rs` | 1 | carga de dados | o CSV é lido e os tipos das colunas são inferidos |
| `type_checker_test.rs` | 52 | semântica | tabelas, colunas e tipos; visibilidade e escopo das CTEs; nomes ambíguos |
| `interpreter_test.rs` | 41 | execução | LOAD, DESCRIBE, WHERE, NULL, JOIN, CTEs, trace |
| `pipeline_test.rs` | 20 | ponta a ponta | texto SQL até a saída, com cada erro detectado na fase correta |

Nos testes de parser, a maioria dos casos aparece em par: uma versão com palavras-chave em maiúsculas e outra com o sufixo `_minus`, em minúsculas.

### Isolamento das fases

Os testes do type checker e do interpretador **montam a AST à mão**, sem passar pelo parser. Assim, uma falha nesses arquivos aponta para a fase testada, e não para a gramática ou para o `ast_builder`. Apenas o `pipeline_test.rs` percorre todas as fases a partir do texto SQL. Ele também verifica que cada erro sai com o prefixo da fase em que foi detectado (`Erro de sintaxe`, `Erro semântico` ou `Erro de execução`).

### Dados de teste

| Arquivo | Colunas | Uso |
|---|---|---|
| `test_data/alunos.csv` | `nome` (String), `idade` (Int), `curso` (String) | carga do CSV e consultas simples |
| `test_data/users.csv` | `name` (String), `age` (Int), `type` (String), `credit` (Float), `active` (Boolean) | type checker, interpretador, pipeline e CTEs |
| `test_data/usuarios.csv` | `id` (Int), `nome` (String), `idade` (Int) | JOIN e nomes qualificados |
| `test_data/pedidos.csv` | `id` (Int), `usuario_id` (Int), `produto` (String), `valor` (Float) | JOIN e nomes qualificados |
| `test_data/disciplinas.csv` | `disciplina` (String), `codigo` (String), `creditos` (Int) | testes manuais |

O `users.csv` cobre os quatro tipos da linguagem e tem uma célula de `credit` vazia, usada nos testes de NULL. `usuarios` e `pedidos` têm de propósito uma coluna `id` em comum, e cada um tem uma linha sem correspondência no outro (Carlos não tem pedido; o pedido 104 é de um usuário que não existe), o que diferencia `INNER` de `LEFT JOIN`.

### Critério de aceitação nos testes de parser

Por causa da recuperação de erro do ANTLR (seção 5), `parser.program()` devolve `Ok` mesmo com erro de sintaxe. Em `form users select name;` (com `FROM` digitado errado), o parser reporta

```
line 1:0 mismatched input 'form' expecting {FROM, LOAD, DESCRIBE, WITH}
```

e mesmo assim `program()` retorna `Ok`. Só a consulta ao contador `number_of_syntax_errors()` revela o problema. Por isso os dois arquivos de teste de parser usam o contador:

```rust
// parser_error_test.rs: a consulta precisa ter algum erro
result.is_err() || parser.number_of_syntax_errors() > 0

// parser_success_test.rs: a consulta precisa ter zero erros
result.is_ok() && parser.number_of_syntax_errors() == 0
```

O `parser_success_test.rs` usava apenas `is_ok()`, e portanto nenhum de seus testes podia falhar. Corrigido o critério, apareceram dois testes errados (`test_negative_numbers` e `_minus`, sem o `;` final), também corrigidos.

### Casos representativos

| Teste | Conceito que demonstra |
|---|---|
| `test_invalid_identifier_start` | erro léxico × erro sintático; necessidade do `ERROR_CHARACTER` |
| `test_typo_in_from_minus` | recuperação de erro do ANTLR e por que `is_err()` não basta |
| `test_select_with_not_and_parenthesis` | recursão da regra `condition` e agrupamento por parênteses |
| `test_comments_and_whitespaces` | descarte de espaços e comentários pelo lexer (`-> skip`) |
| `valor_faltando_nao_e_inventado` | uma árvore "consertada" pelo ANTLR não é executada |
| `cte_nao_enxerga_as_ctes_seguintes` | escopo das CTEs pela ordem de inserção no ambiente |
| `cte_nao_existe_fora_da_sua_query` | CTE como tabela temporária: o banco não é alterado |
| `mesma_coluna_dos_dois_lados_e_ambigua` | resolução de nomes e o bug do produto cartesiano |
| `where_sobre_coluna_nula_do_left_join_nao_quebra` | NULL do LEFT JOIN segue a mesma convenção do CSV |
| `trace_mostra_criacao_e_descarte_das_ctes` | ciclo de vida das tabelas temporárias |

---

## 9. Como executar

Pré-requisitos: [Rust](https://rustup.rs) (edição 2024).

```bash
cd mini_sql
```

### Terminal interativo

```bash
cargo run
```

```
MiniSQL> LOAD 'test_data/users.csv';
Tabela 'users' carregada.
MiniSQL> DESC users;
coluna | tipo
-------+--------
name   | String
age    | Int
type   | String
credit | Float
active | Boolean
(5 colunas, 5 linhas)
MiniSQL> FROM users SELECT name, credit WHERE type IN ('admin', 'editor') AND active = TRUE AND credit >= 150.50;
name  | credit
------+-------
ana   | 200.75
elisa | 310.0
(2 linhas)
MiniSQL> FROM users SELECT name WHERE age = 'vinte';
Erro semântico: Tipos incompatíveis em `age = 'vinte'`: a coluna 'age' é Int e o literal 'vinte' é String
MiniSQL> FROM users SELECT name
Erro de sintaxe: linha 1, coluna 23: missing ';' at '<EOF>'
MiniSQL> exit
```

### JOIN e nomes qualificados

```
MiniSQL> LOAD 'test_data/usuarios.csv';
Tabela 'usuarios' carregada.
MiniSQL> LOAD 'test_data/pedidos.csv';
Tabela 'pedidos' carregada.
MiniSQL> FROM usuarios LEFT JOIN pedidos ON usuarios.id = pedidos.usuario_id SELECT nome, produto, valor;
nome   | produto | valor
-------+---------+------
Ana    | Teclado | 150.0
Ana    | Mouse   | 80.0
Bruno  | Monitor | 900.0
Carlos |         |
(4 linhas)
MiniSQL> FROM usuarios JOIN pedidos ON id = usuario_id SELECT nome;
Erro semântico: Coluna 'id' é ambígua: existe em usuarios e pedidos; escreva usuarios.id ou pedidos.id
```

### CTEs com `.trace`

```
MiniSQL> .trace
Trace ligado.
MiniSQL> WITH adultos (nome, anos) AS (FROM users SELECT name, age WHERE age >= 18) FROM adultos SELECT nome WHERE anos > 30;
[CTE] tabela temporária 'adultos' criada:
nome  | anos
------+-----
ana   | 20
carla | 35
diego | 42
elisa | 28
(4 linhas)
[CTE] tabelas temporárias descartadas: adultos
nome
-----
carla
diego
(2 linhas)
MiniSQL> FROM adultos SELECT nome;
Erro semântico: Tabela 'adultos' não encontrada (use LOAD 'arquivo.csv'; antes de consultá-la)
```

A última linha mostra o escopo funcionando: a tabela temporária deixou de existir junto com a consulta.

### Testes

```bash
cargo build                                  # compila
cargo test                                   # todos
cargo test --test type_checker_test          # só um arquivo
cargo test --test interpreter_test where_in  # só os testes cujo nome contém "where_in"
```

Para inspecionar a AST de uma consulta, o teste da AST a imprime:

```bash
cargo test --test ast_test -- --nocapture
```

### Regenerar o analisador

Necessário apenas após alterar a gramática:

```bash
cargo install antlr-rust-codegen --bin antlr4-rust-gen   # uma única vez
cd mini_sql
antlr4-rust-gen --visitor --out-dir generated grammar/miniSQL.g4
```

No Git Bash, use barra normal no caminho — a contrabarra é interpretada como escape.

---

## 10. Limitações conhecidas

- **Sem apelidos de tabela.** `FROM users JOIN users ...` é detectado e recusado (`a tabela 'users' aparece mais de uma vez`), mas não é suportado.
- **`ON` com uma única comparação.** A regra é `ON expr`, e não `ON condition`; não é possível escrever `AND` no `ON`.
- **Sem `IS NULL`.** Não há como selecionar as linhas sem correspondência de um `LEFT JOIN`.
- **Mensagens de erro de sintaxe em inglês.** São as mensagens do ANTLR (`missing ';' at '<EOF>'`), com os nomes internos dos tokens; só a linha e a coluna são formatadas por nós.
- **Leitura de CSV simplificada.** As linhas são separadas por `,` sem tratar aspas: um valor como `"Silva, Ana"` quebra a linha.
- **JOIN O(n × m).** O *nested loop join* compara todas as linhas com todas; suficiente para o tamanho das tabelas do projeto.
- **`Condition::Parens` preserva os parênteses na AST.** Como eles já não têm efeito sobre a estrutura da árvore, são informação redundante.

## 11. Próximos passos

- funções básicas de SQL (agregação)
- `ORDER BY`
- `GROUP BY`
- operadores algébricos
- apelidos de tabela (`FROM users AS u`)

---

## Tecnologias

- **Rust** (edição 2024) e **Cargo**
- **ANTLR4** para a definição da gramática
- [`antlr-rust-runtime` / `antlr-rust-codegen`](https://github.com/ophi-dev/antlr-rust-runtime) 0.34.0 — implementação independente em Rust que gera os reconhecedores a partir do `.g4` e os executa, sem dependência de Java
