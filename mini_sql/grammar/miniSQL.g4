grammar miniSQL;


// () obrigatório
// ()* 0 ou + vezes
// ()? 0 ou 1 vez
// ()+ 1 ou + vezes


// REGRAS DE PARSER

program : query+ EOF;

query : FROM ID SELECT selectList  (WHERE condition)? END;


// Permite selecionar múltiplos campos (ex: a, b, c) ou '*'
selectList : ID (',' ID)*
           | '*' ;

// permite IN no WHERE
expressaoIn : value IN '(' value(',' value)*')';


// Suporta condições encadeadas por AND / OR (ex: a = 1 AND b > 2)
// mas eles nâo podem ficar na mesma linha, SQL tem preferência pelo AND
condition : NOT condition
          |condition AND condition //// resolver com parenteses?????
          | condition OR condition
          | '(' condition ')'
          | expr
          | expressaoIn ;

expr : left=value op=(EQUAL | NOT_EQUAL | LESS | LESS_EQUAL | GREATER | GREATER_EQUAL) right=value;



//ACHO QUE DESSE JEITO ACEITARIA 1=1 E RETORNARIA TODA, QUEREMOS ? TYPE CHECKER
value: ID
     | INT
     | FLOAT
     | STRING
     | BOOLEAN ;
    


// REGRAS DE LEXER ---> tokens


//palavras chaves
SELECT : 'SELECT' | 'select';
WHERE : 'WHERE' | 'where';
FROM : 'FROM' | 'from';


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

END: ';';
// ws
NEWLINE : [ \t\r\n]+ -> skip ;

// fragment
// Um fragment não gera um token sozinho. Ele serve como um pedaço reutilizável para construir outros tokens.
fragment DIGIT : [0-9] ;

// ~ == negação
COMMENT : '--' ~[\r\n]* -> skip ;

//porque
ERROR_CHARACTER : . ;

// Se começa com maiúscula, é token do Lexer. Se começa com minúscula, é regra do Parser.