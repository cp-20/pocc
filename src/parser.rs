use crate::ast::{ASTKind, ASTNode, BinaryOp, Parameter, Type, UnaryOp};
use crate::lexer::{LexError, Lexer, Token};
use std::fmt;

#[derive(Debug, Clone)]
enum DeclaratorType {
    Direct,
    Pointer(Box<DeclaratorType>),
    Function(Vec<Parameter>),
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        ParseError {
            message: error.message,
            line: error.line,
            column: error.column,
        }
    }
}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
    line: usize,
    column: usize,
}

impl Parser {
    pub fn new(mut lexer: Lexer) -> Result<Self, ParseError> {
        let current_token = lexer.next_token()?;
        Ok(Parser {
            lexer,
            current_token,
            line: 1,
            column: 1,
        })
    }

    fn advance(&mut self) -> Result<(), ParseError> {
        self.current_token = self.lexer.next_token()?;
        Ok(())
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.advance()
        } else {
            Err(ParseError {
                message: format!(
                    "Expected {expected:?}, found {current:?}",
                    current = self.current_token
                ),
                line: self.line,
                column: self.column,
            })
        }
    }

    fn match_token(&self, token: &Token) -> bool {
        std::mem::discriminant(&self.current_token) == std::mem::discriminant(token)
    }

    pub fn parse(&mut self) -> Result<ASTNode, ParseError> {
        self.parse_translation_unit()
    }

    // translation_unit: external_declaration+
    fn parse_translation_unit(&mut self) -> Result<ASTNode, ParseError> {
        let mut declarations = Vec::new();

        while !self.match_token(&Token::Eof) {
            declarations.push(self.parse_external_declaration()?);
        }

        Ok(ASTNode::new(ASTKind::TranslationUnit(declarations)))
    }

    // external_declaration: function_definition | declaration
    fn parse_external_declaration(&mut self) -> Result<ASTNode, ParseError> {
        // Look ahead to determine if this is a function definition or declaration
        // For simplicity, we'll parse the type_specifier and declarator first
        let type_spec = self.parse_type_specifier()?;
        let declarator_info = self.parse_declarator_info()?;

        if self.match_token(&Token::LeftBrace) {
            // Function definition
            self.parse_function_definition_with_info(type_spec, declarator_info)
        } else {
            // Declaration
            self.expect(Token::Semicolon)?;
            Ok(ASTNode::new(ASTKind::Declaration {
                var_type: self.apply_declarator_to_type(declarator_info.1, type_spec),
                name: declarator_info.0,
            }))
        }
    }

    fn parse_function_definition_with_info(
        &mut self,
        ret_type: Type,
        declarator_info: (String, DeclaratorType),
    ) -> Result<ASTNode, ParseError> {
        let function_type = self.apply_declarator_to_type(declarator_info.1, ret_type.clone());

        let (name, params) = match function_type {
            Type::Function {
                ret_type: _,
                params,
            } => (declarator_info.0, params),
            _ => {
                return Err(ParseError {
                    message: "Expected function type".to_string(),
                    line: self.line,
                    column: self.column,
                });
            }
        };

        let body = self.parse_compound_statement()?;

        Ok(ASTNode::new(ASTKind::FunctionDefinition {
            ret_type,
            name,
            params,
            body: Box::new(body),
        }))
    }

    // type_specifier: "void" | "char" | "int" | "long"
    fn parse_type_specifier(&mut self) -> Result<Type, ParseError> {
        let mut base_type: Type;
        match &self.current_token {
            Token::Void => {
                self.advance()?;
                base_type = Type::Void;
            }
            Token::Char => {
                self.advance()?;
                base_type = Type::Char;
            }
            Token::Int => {
                self.advance()?;
                base_type = Type::Int;
            }
            Token::Long => {
                self.advance()?;
                base_type = Type::Long;
            }
            _ => {
                return Err(ParseError {
                    message: format!(
                        "Expected type specifier, found {current_token:?}",
                        current_token = self.current_token
                    ),
                    line: self.line,
                    column: self.column,
                });
            }
        }
        while self.match_token(&Token::Multiply) {
            self.advance()?;
            base_type = Type::Pointer(Box::new(base_type));
        }
        Ok(base_type)
    }

    // Simplified declarator parsing - returns (name, declarator_type)
    fn parse_declarator_info(&mut self) -> Result<(String, DeclaratorType), ParseError> {
        self.parse_declarator_info_impl()
    }

    fn parse_declarator_info_impl(&mut self) -> Result<(String, DeclaratorType), ParseError> {
        if self.match_token(&Token::LeftParen) {
            // Could be parenthesized declarator or function
            self.advance()?; // consume '('

            if self.match_token(&Token::RightParen) {
                // Function with no parameters
                self.advance()?; // consume ')'
                let (name, _inner_type) = self.parse_declarator_info_impl()?;
                Ok((name, DeclaratorType::Function(vec![])))
            } else {
                // Check if this is a parameter list or parenthesized declarator
                // For simplicity, assume it's a function parameter list if we see a type specifier
                if self.is_type_specifier() {
                    let params = self.parse_parameter_list()?;
                    self.expect(Token::RightParen)?;
                    let (name, _inner_type) = self.parse_declarator_info_impl()?;
                    Ok((name, DeclaratorType::Function(params)))
                } else {
                    // Parenthesized declarator
                    let (name, inner_type) = self.parse_declarator_info_impl()?;
                    self.expect(Token::RightParen)?;
                    Ok((name, inner_type))
                }
            }
        } else if let Token::Identifier(name) = &self.current_token {
            let name = name.clone();
            self.advance()?;

            // Check for function parameters
            if self.match_token(&Token::LeftParen) {
                self.advance()?; // consume '('
                if self.match_token(&Token::RightParen) {
                    self.advance()?; // consume ')'
                    Ok((name, DeclaratorType::Function(vec![])))
                } else {
                    let params = self.parse_parameter_list()?;
                    self.expect(Token::RightParen)?;
                    Ok((name, DeclaratorType::Function(params)))
                }
            } else {
                Ok((name, DeclaratorType::Direct))
            }
        } else {
            Err(ParseError {
                message: format!(
                    "Expected declarator, found {current_token:?}",
                    current_token = self.current_token
                ),
                line: self.line,
                column: self.column,
            })
        }
    }

    fn is_type_specifier(&self) -> bool {
        matches!(
            self.current_token,
            Token::Void | Token::Char | Token::Int | Token::Long
        )
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut params = Vec::new();

        loop {
            let param_type = self.parse_type_specifier()?;
            let (param_name, declarator_type) = self.parse_declarator_info()?;
            params.push(Parameter {
                name: param_name,
                param_type: self.apply_declarator_to_type(declarator_type, param_type),
            });

            if self.match_token(&Token::Comma) {
                self.advance()?;
            } else {
                break;
            }
        }

        Ok(params)
    }

    fn apply_declarator_to_type(&self, declarator: DeclaratorType, base_type: Type) -> Type {
        match declarator {
            DeclaratorType::Direct => base_type,
            DeclaratorType::Pointer(inner) => {
                Type::Pointer(Box::new(self.apply_declarator_to_type(*inner, base_type)))
            }
            DeclaratorType::Function(params) => Type::Function {
                ret_type: Box::new(base_type),
                params,
            },
        }
    }

    // compound_statement: "{" declaration_list statement_list "}"
    fn parse_compound_statement(&mut self) -> Result<ASTNode, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut declarations = Vec::new();
        let mut statements = Vec::new();

        // Parse declarations first
        while self.is_type_specifier() {
            declarations.push(self.parse_declaration()?);
        }

        // Parse statements
        while !self.match_token(&Token::RightBrace) && !self.match_token(&Token::Eof) {
            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RightBrace)?;

        Ok(ASTNode::new(ASTKind::CompoundStatement {
            declarations,
            statements,
        }))
    }

    fn parse_declaration(&mut self) -> Result<ASTNode, ParseError> {
        let type_spec = self.parse_type_specifier()?;
        let (name, declarator_type) = self.parse_declarator_info()?;
        self.expect(Token::Semicolon)?;

        Ok(ASTNode::new(ASTKind::Declaration {
            var_type: self.apply_declarator_to_type(declarator_type, type_spec),
            name,
        }))
    }

    fn parse_statement(&mut self) -> Result<ASTNode, ParseError> {
        match &self.current_token {
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::Return => self.parse_return_statement(),
            Token::LeftBrace => self.parse_compound_statement(),
            Token::Identifier(_) => self.parse_expression_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_if_statement(&mut self) -> Result<ASTNode, ParseError> {
        self.expect(Token::If)?;
        self.expect(Token::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParen)?;
        let then_stmt = self.parse_statement()?;

        let else_stmt = if self.match_token(&Token::Else) {
            self.advance()?;
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(ASTNode::new(ASTKind::IfStatement {
            condition: Box::new(condition),
            then_stmt: Box::new(then_stmt),
            else_stmt,
        }))
    }

    fn parse_while_statement(&mut self) -> Result<ASTNode, ParseError> {
        self.expect(Token::While)?;
        self.expect(Token::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(Token::RightParen)?;
        let body = self.parse_statement()?;

        Ok(ASTNode::new(ASTKind::WhileStatement {
            condition: Box::new(condition),
            body: Box::new(body),
        }))
    }

    fn parse_return_statement(&mut self) -> Result<ASTNode, ParseError> {
        self.expect(Token::Return)?;

        let expr = if self.match_token(&Token::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        self.expect(Token::Semicolon)?;

        Ok(ASTNode::new(ASTKind::ReturnStatement(expr)))
    }

    fn parse_expression_statement(&mut self) -> Result<ASTNode, ParseError> {
        let expr = if self.match_token(&Token::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };

        self.expect(Token::Semicolon)?;

        Ok(ASTNode::new(ASTKind::ExpressionStatement(expr)))
    }

    // Expression parsing with operator precedence
    fn parse_expression(&mut self) -> Result<ASTNode, ParseError> {
        self.parse_assignment_expression()
    }

    // assignment_expression: logical_or_expression ("=" assignment_expression)?
    fn parse_assignment_expression(&mut self) -> Result<ASTNode, ParseError> {
        let left = self.parse_logical_or_expression()?;

        if self.match_token(&Token::Assign) {
            self.advance()?;
            let right = self.parse_assignment_expression()?;
            Ok(ASTNode::new(ASTKind::BinaryExpression {
                op: BinaryOp::Assign,
                left: Box::new(left),
                right: Box::new(right),
            }))
        } else {
            Ok(left)
        }
    }

    // logical_or_expression: logical_and_expression ("||" logical_and_expression)*
    fn parse_logical_or_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_logical_and_expression()?;

        while self.match_token(&Token::Or) {
            self.advance()?;
            let right = self.parse_logical_and_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // logical_and_expression: equality_expression ("&&" equality_expression)*
    fn parse_logical_and_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_equality_expression()?;

        while self.match_token(&Token::And) {
            self.advance()?;
            let right = self.parse_equality_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // equality_expression: relational_expression ("==" relational_expression)*
    fn parse_equality_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_relational_expression()?;

        while self.match_token(&Token::Eq) {
            self.advance()?;
            let right = self.parse_relational_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op: BinaryOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // relational_expression: additive_expression ("<" additive_expression)*
    fn parse_relational_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_additive_expression()?;

        while self.match_token(&Token::Less) {
            self.advance()?;
            let right = self.parse_additive_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op: BinaryOp::Less,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // additive_expression: multiplicative_expression (("+" | "-") multiplicative_expression)*
    fn parse_additive_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_multiplicative_expression()?;

        while self.match_token(&Token::Plus) || self.match_token(&Token::Minus) {
            let op = if self.match_token(&Token::Plus) {
                BinaryOp::Add
            } else {
                BinaryOp::Sub
            };
            self.advance()?;
            let right = self.parse_multiplicative_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // multiplicative_expression: unary_expression (("*" | "/") unary_expression)*
    fn parse_multiplicative_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut left = self.parse_unary_expression()?;

        while self.match_token(&Token::Multiply) || self.match_token(&Token::Divide) {
            let op = if self.match_token(&Token::Multiply) {
                BinaryOp::Mul
            } else {
                BinaryOp::Div
            };
            self.advance()?;
            let right = self.parse_unary_expression()?;
            left = ASTNode::new(ASTKind::BinaryExpression {
                op,
                left: Box::new(left),
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    // unary_expression: unary_operator unary_expression | postfix_expression
    fn parse_unary_expression(&mut self) -> Result<ASTNode, ParseError> {
        match &self.current_token {
            Token::Address => {
                self.advance()?;
                let operand = self.parse_unary_expression()?;
                Ok(ASTNode::new(ASTKind::UnaryExpression {
                    op: UnaryOp::Address,
                    operand: Box::new(operand),
                }))
            }
            Token::Multiply => {
                self.advance()?;
                let operand = self.parse_unary_expression()?;
                Ok(ASTNode::new(ASTKind::UnaryExpression {
                    op: UnaryOp::Deref,
                    operand: Box::new(operand),
                }))
            }
            Token::Plus => {
                self.advance()?;
                let operand = self.parse_unary_expression()?;
                Ok(ASTNode::new(ASTKind::UnaryExpression {
                    op: UnaryOp::Plus,
                    operand: Box::new(operand),
                }))
            }
            Token::Minus => {
                self.advance()?;
                let operand = self.parse_unary_expression()?;
                Ok(ASTNode::new(ASTKind::UnaryExpression {
                    op: UnaryOp::Minus,
                    operand: Box::new(operand),
                }))
            }
            Token::Not => {
                self.advance()?;
                let operand = self.parse_unary_expression()?;
                Ok(ASTNode::new(ASTKind::UnaryExpression {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                }))
            }
            _ => self.parse_postfix_expression(),
        }
    }

    // postfix_expression: primary_expression ("(" argument_expression_list? ")")*
    fn parse_postfix_expression(&mut self) -> Result<ASTNode, ParseError> {
        let mut expr = self.parse_primary_expression()?;

        while self.match_token(&Token::LeftParen) {
            self.advance()?; // consume '('

            let args = if self.match_token(&Token::RightParen) {
                Vec::new()
            } else {
                self.parse_argument_expression_list()?
            };

            self.expect(Token::RightParen)?;

            expr = ASTNode::new(ASTKind::FunctionCall {
                function: Box::new(expr),
                args,
            });
        }

        Ok(expr)
    }

    fn parse_argument_expression_list(&mut self) -> Result<Vec<ASTNode>, ParseError> {
        let mut args = Vec::new();

        loop {
            args.push(self.parse_assignment_expression()?);

            if self.match_token(&Token::Comma) {
                self.advance()?;
            } else {
                break;
            }
        }

        Ok(args)
    }

    // primary_expression: identifier | constant | string | "(" expression ")"
    fn parse_primary_expression(&mut self) -> Result<ASTNode, ParseError> {
        match &self.current_token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                Ok(ASTNode::new(ASTKind::Identifier(name)))
            }
            Token::Integer(value) => {
                let value = *value;
                self.advance()?;
                Ok(ASTNode::new(ASTKind::IntegerLiteral(value)))
            }
            Token::Character(ch) => {
                let ch = *ch;
                self.advance()?;
                Ok(ASTNode::new(ASTKind::CharacterLiteral(ch)))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(ASTNode::new(ASTKind::StringLiteral(s)))
            }
            Token::LeftParen => {
                self.advance()?; // consume '('
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(ASTNode::new(ASTKind::ParenExpression(Box::new(expr))))
            }
            _ => Err(ParseError {
                message: format!(
                    "Unexpected token in expression: {current_token:?}",
                    current_token = self.current_token
                ),
                line: self.line,
                column: self.column,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_simple_function() {
        let input = "int main() { return 0; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        let expected = ASTNode::new(ASTKind::TranslationUnit(vec![ASTNode::new(
            ASTKind::FunctionDefinition {
                ret_type: Type::Int,
                name: "main".to_string(),
                params: vec![],
                body: Box::new(ASTNode::new(ASTKind::CompoundStatement {
                    declarations: vec![],
                    statements: vec![ASTNode::new(ASTKind::ReturnStatement(Some(Box::new(
                        ASTNode::new(ASTKind::IntegerLiteral(0)),
                    ))))],
                })),
            },
        )]));

        assert_eq!(ast, expected);
    }

    #[test]
    fn test_expression() {
        let input = "int test() { x = 1 + 2 * 3; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        let expected = ASTNode::new(ASTKind::TranslationUnit(vec![ASTNode::new(
            ASTKind::FunctionDefinition {
                ret_type: Type::Int,
                name: "test".to_string(),
                params: vec![],
                body: Box::new(ASTNode::new(ASTKind::CompoundStatement {
                    declarations: vec![],
                    statements: vec![ASTNode::new(ASTKind::ExpressionStatement(Some(Box::new(
                        ASTNode::new(ASTKind::BinaryExpression {
                            op: BinaryOp::Assign,
                            left: Box::new(ASTNode::new(ASTKind::Identifier("x".to_string()))),
                            right: Box::new(ASTNode::new(ASTKind::BinaryExpression {
                                op: BinaryOp::Add,
                                left: Box::new(ASTNode::new(ASTKind::IntegerLiteral(1))),
                                right: Box::new(ASTNode::new(ASTKind::BinaryExpression {
                                    op: BinaryOp::Mul,
                                    left: Box::new(ASTNode::new(ASTKind::IntegerLiteral(2))),
                                    right: Box::new(ASTNode::new(ASTKind::IntegerLiteral(3))),
                                })),
                            })),
                        }),
                    ))))],
                })),
            },
        )]));

        assert_eq!(ast, expected);
    }

    #[test]
    fn test_malloc() {
        let input = "void* malloc();";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        let expected = ASTNode::new(ASTKind::TranslationUnit(vec![ASTNode::new(
            ASTKind::Declaration {
                var_type: Type::Function {
                    ret_type: Box::new(Type::Pointer(Box::new(Type::Void))),
                    params: vec![],
                },
                name: "malloc".to_string(),
            },
        )]));

        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_with_params() {
        let input = "void func(int a, char b) { return; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        let expected = ASTNode::new(ASTKind::TranslationUnit(vec![ASTNode::new(
            ASTKind::FunctionDefinition {
                ret_type: Type::Void,
                name: "func".to_string(),
                params: vec![
                    Parameter {
                        name: "a".to_string(),
                        param_type: Type::Int,
                    },
                    Parameter {
                        name: "b".to_string(),
                        param_type: Type::Char,
                    },
                ],
                body: Box::new(ASTNode::new(ASTKind::CompoundStatement {
                    declarations: vec![],
                    statements: vec![ASTNode::new(ASTKind::ReturnStatement(None))],
                })),
            },
        )]));

        assert_eq!(ast, expected);
    }

    #[test]
    fn test_function_call() {
        let input = "void call() { func(1, 'a'); }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        let expected = ASTNode::new(ASTKind::TranslationUnit(vec![ASTNode::new(
            ASTKind::FunctionDefinition {
                ret_type: Type::Void,
                name: "call".to_string(),
                params: vec![],
                body: Box::new(ASTNode::new(ASTKind::CompoundStatement {
                    declarations: vec![],
                    statements: vec![ASTNode::new(ASTKind::ExpressionStatement(Some(Box::new(
                        ASTNode::new(ASTKind::FunctionCall {
                            function: Box::new(ASTNode::new(ASTKind::Identifier(
                                "func".to_string(),
                            ))),
                            args: vec![
                                ASTNode::new(ASTKind::IntegerLiteral(1)),
                                ASTNode::new(ASTKind::CharacterLiteral('a')),
                            ],
                        }),
                    ))))],
                })),
            },
        )]));

        assert_eq!(ast, expected);
    }
}
