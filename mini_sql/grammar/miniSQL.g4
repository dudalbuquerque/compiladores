grammar miniSQL;


// () obrigatório
// ()* 0 ou + vezes
// ()? 0 ou 1 vez
// ()+ 1 ou + vezes

// ~ == negação


// REGRAS DE PARSER

program : statement+ EOF;

statement: query
          | load
          | describe ;

//permite importar CSV
load: LOAD STRING END;

// permite descrever infos do arquivo csv/tabela
describe: DESCRIBE ID END ;

// CTEs
query: cteClause? queryBody END ;

// O COMMA SIGNIFICA QUE PODE TER UMA VÍRGULA SEGUIDA DE OUTRAS CTEs, dentro do mesmo With
cteClause: WITH cte (COMMA cte)* ;

// Ex: cte_nome AS (FROM ...) ou cte_nome (col1, col2) AS (FROM ...)
cte: ID (LPAREN ID (COMMA ID)* RPAREN)? AS LPAREN queryBody RPAREN ;

// Corpo principal da consulta sem o ; final, para poder ser reutilizado dentro de CTEs
queryBody: FROM ID (join)* SELECT selectList (WHERE condition)? ;

// Permite selecionar múltiplos campos (ex: a, b, c) ou '*'
selectList : columnRef (COMMA columnRef)*
           | STAR ;

// Referência a uma coluna: `coluna` ou `tabela.coluna`.
// O prefixo é necessário quando duas tabelas da query (FROM/JOIN/CTE) têm
// uma coluna com o mesmo nome, ex.: `usuarios.id = pedidos.usuario_id`.
columnRef : ID (DOT ID)? ;

// permite IN no WHERE
expressaoIn : value IN LPAREN value (COMMA value)* RPAREN ;

// joins encadeados (ex: JOIN a ON a.id = b.id JOIN c ON c.id = b.id)
join : (INNER | LEFT)? JOIN ID ON expr ;


// Suporta condições encadeadas por AND / OR (ex: a = 1 AND b > 2)
// mas eles nâo podem ficar na mesma linha, SQL tem preferência pelo AND
//Para converter, o código precisa saber qual alternativa da regra condition foi usada em cada nó (NOT, AND, OR, parênteses, comparação ou IN)
//Sem rótulos: o nó raiz é um ConditionContext genérico, e você descobre que é um AND procurando se existe o token AND dentro dele.
//Com rótulos: o nó raiz já vem como AndContext, e o código faz match direto.

condition : NOT inner=condition                  # not
          | left=condition AND right=condition   # and
          | left=condition OR right=condition    # or
          | LPAREN inner=condition RPAREN        # parens
          | expr                                 # exprCond
          | expressaoIn                          # inCond
          ;


expr : left=value op=(EQUAL | NOT_EQUAL | LESS | LESS_EQUAL | GREATER | GREATER_EQUAL) right=value;


value: columnRef
     | INT
     | FLOAT
     | STRING
     | BOOLEAN ;
    


// REGRAS DE LEXER ---> tokens


//palavras chaves
SELECT : 'SELECT' | 'select';
WHERE : 'WHERE' | 'where';
FROM : 'FROM' | 'from';
JOIN : 'JOIN' | 'join';
INNER : 'INNER' | 'inner';
LEFT : 'LEFT' | 'left';
ON : 'ON' | 'on';
LOAD : 'LOAD' | 'load';
DESCRIBE : 'DESCRIBE' | 'describe' | 'DESC' | 'desc';
WITH : 'WITH' | 'with';
AS : 'AS' | 'as';

// operadores lógicos, precisam estar antes do ID por regra do lexer
AND : 'AND' | 'and';
OR : 'OR' | 'or';
IN: 'IN' | 'in';
NOT: 'NOT' | 'not';

// types e id
BOOLEAN : 'TRUE' | 'FALSE' | 'true' | 'false'  ;
ID : [a-z] [a-z0-9_]* ;
INT : '-'? DIGIT+ ;
FLOAT : '-'? DIGIT+ '.' DIGIT+ ;
STRING  : '\'' ~[\r\n';]* '\'' ;


//operators comparação
EQUAL : '=' ;
NOT_EQUAL : '!=' | '<>' ;
LESS : '<' ;
LESS_EQUAL : '<=' ;
GREATER : '>' ;
GREATER_EQUAL : '>=' ;


// pontuação
COMMA  : ',' ;
DOT    : '.' ;
STAR   : '*' ;
LPAREN : '(' ;
RPAREN : ')' ;
END    : ';' ;

NEWLINE : [ \t\r\n]+ -> skip ;
COMMENT : '--' ~[\r\n]* -> skip ;

// fragment
// Um fragment não gera um token sozinho. Ele serve como um pedaço reutilizável para construir outros tokens.
fragment DIGIT : [0-9] ;


//não casa com id e da erro no parser, para de dar erro no lexer
ERROR_CHARACTER : . ;

// Se começa com maiúscula, é token do Lexer. Se começa com minúscula, é regra do Parser.