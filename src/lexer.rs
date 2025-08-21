use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Char,
    Else,
    If,
    Int,
    Long,
    Return,
    Void,
    While,

    // Operators
    Eq,       // ==
    And,      // &&
    Or,       // ||
    Plus,     // +
    Minus,    // -
    Multiply, // *
    Divide,   // /
    Less,     // <
    Assign,   // =
    Address,  // &
    Not,      // !

    // Separators
    Semicolon,  // ;
    Colon,      // :
    LeftBrace,  // {
    RightBrace, // }
    Comma,      // ,
    LeftParen,  // (
    RightParen, // )

    // Literals and identifiers
    Identifier(String),
    Integer(i64),
    Character(char),
    String(String),

    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Char => write!(f, "char"),
            Token::Else => write!(f, "else"),
            Token::If => write!(f, "if"),
            Token::Int => write!(f, "int"),
            Token::Long => write!(f, "long"),
            Token::Return => write!(f, "return"),
            Token::Void => write!(f, "void"),
            Token::While => write!(f, "while"),
            Token::Eq => write!(f, "=="),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Multiply => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::Less => write!(f, "<"),
            Token::Assign => write!(f, "="),
            Token::Address => write!(f, "&"),
            Token::Not => write!(f, "!"),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Comma => write!(f, ","),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Identifier(s) => write!(f, "identifier({s})"),
            Token::Integer(n) => write!(f, "integer({n})"),
            Token::Character(c) => write!(f, "character({c})"),
            Token::String(s) => write!(f, "string({s})"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lexical error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments()?;

        if self.position >= self.input.len() {
            return Ok(Token::Eof);
        }

        let ch = self.input[self.position];

        match ch {
            // Two-character operators
            '=' if self.peek() == Some('=') => {
                self.advance();
                self.advance();
                Ok(Token::Eq)
            }
            '&' if self.peek() == Some('&') => {
                self.advance();
                self.advance();
                Ok(Token::And)
            }
            '|' if self.peek() == Some('|') => {
                self.advance();
                self.advance();
                Ok(Token::Or)
            }

            // Single-character operators and separators
            ';' => {
                self.advance();
                Ok(Token::Semicolon)
            }
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            '{' => {
                self.advance();
                Ok(Token::LeftBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::RightBrace)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            '=' => {
                self.advance();
                Ok(Token::Assign)
            }
            '(' => {
                self.advance();
                Ok(Token::LeftParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RightParen)
            }
            '&' => {
                self.advance();
                Ok(Token::Address)
            }
            '!' => {
                self.advance();
                Ok(Token::Not)
            }
            '-' => {
                self.advance();
                Ok(Token::Minus)
            }
            '+' => {
                self.advance();
                Ok(Token::Plus)
            }
            '*' => {
                self.advance();
                Ok(Token::Multiply)
            }
            '/' => {
                self.advance();
                Ok(Token::Divide)
            }
            '<' => {
                self.advance();
                Ok(Token::Less)
            }

            // Character literals
            '\'' => self.read_character_literal(),

            // String literals
            '"' => self.read_string_literal(),

            // Numbers
            '0'..='9' => self.read_number(),

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier_or_keyword(),

            _ => Err(LexError {
                message: format!("Unknown character: '{ch}'"),
                line: self.line,
                column: self.column,
            }),
        }
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            if self.input[self.position] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            if self.position >= self.input.len() {
                break;
            }

            let ch = self.input[self.position];

            // Skip whitespace
            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            // Skip comments /* ... */
            if ch == '/' && self.peek() == Some('*') {
                self.advance(); // skip '/'
                self.advance(); // skip '*'

                while self.position + 1 < self.input.len() {
                    if self.input[self.position] == '*' && self.input[self.position + 1] == '/' {
                        self.advance(); // skip '*'
                        self.advance(); // skip '/'
                        break;
                    }
                    self.advance();
                }

                if self.position >= self.input.len() {
                    return Err(LexError {
                        message: "Unterminated comment".to_string(),
                        line: self.line,
                        column: self.column,
                    });
                }
                continue;
            }

            break;
        }
        Ok(())
    }

    fn read_character_literal(&mut self) -> Result<Token, LexError> {
        self.advance(); // skip opening '

        if self.position >= self.input.len() {
            return Err(LexError {
                message: "Unterminated character literal".to_string(),
                line: self.line,
                column: self.column,
            });
        }

        let ch = if self.input[self.position] == '\\' {
            // Escape sequence
            self.advance(); // skip '\'
            if self.position >= self.input.len() {
                return Err(LexError {
                    message: "Unterminated character literal".to_string(),
                    line: self.line,
                    column: self.column,
                });
            }

            match self.input[self.position] {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                c => c,
            }
        } else {
            self.input[self.position]
        };

        self.advance(); // skip character

        if self.position >= self.input.len() || self.input[self.position] != '\'' {
            return Err(LexError {
                message: "Unterminated character literal".to_string(),
                line: self.line,
                column: self.column,
            });
        }

        self.advance(); // skip closing '
        Ok(Token::Character(ch))
    }

    fn read_string_literal(&mut self) -> Result<Token, LexError> {
        self.advance(); // skip opening "
        let mut string = String::new();

        while self.position < self.input.len() && self.input[self.position] != '"' {
            let ch = if self.input[self.position] == '\\' {
                self.advance(); // skip '\'
                if self.position >= self.input.len() {
                    return Err(LexError {
                        message: "Unterminated string literal".to_string(),
                        line: self.line,
                        column: self.column,
                    });
                }

                match self.input[self.position] {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    c => c,
                }
            } else {
                self.input[self.position]
            };

            string.push(ch);
            self.advance();
        }

        if self.position >= self.input.len() {
            return Err(LexError {
                message: "Unterminated string literal".to_string(),
                line: self.line,
                column: self.column,
            });
        }

        self.advance(); // skip closing "
        Ok(Token::String(string))
    }

    fn read_number(&mut self) -> Result<Token, LexError> {
        let mut number = String::new();

        while self.position < self.input.len() && self.input[self.position].is_ascii_digit() {
            number.push(self.input[self.position]);
            self.advance();
        }

        match number.parse::<i64>() {
            Ok(n) => Ok(Token::Integer(n)),
            Err(_) => Err(LexError {
                message: format!("Invalid number: {number}"),
                line: self.line,
                column: self.column,
            }),
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Result<Token, LexError> {
        let mut identifier = String::new();

        while self.position < self.input.len() {
            let ch = self.input[self.position];
            if ch.is_alphanumeric() || ch == '_' {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let token = match identifier.as_str() {
            "char" => Token::Char,
            "else" => Token::Else,
            "if" => Token::If,
            "int" => Token::Int,
            "long" => Token::Long,
            "return" => Token::Return,
            "void" => Token::Void,
            "while" => Token::While,
            _ => Token::Identifier(identifier),
        };

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("int main void");
        assert_eq!(lexer.next_token().unwrap(), Token::Int);
        assert_eq!(
            lexer.next_token().unwrap(),
            Token::Identifier("main".to_string())
        );
        assert_eq!(lexer.next_token().unwrap(), Token::Void);
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("== && || + - * /");
        assert_eq!(lexer.next_token().unwrap(), Token::Eq);
        assert_eq!(lexer.next_token().unwrap(), Token::And);
        assert_eq!(lexer.next_token().unwrap(), Token::Or);
        assert_eq!(lexer.next_token().unwrap(), Token::Plus);
        assert_eq!(lexer.next_token().unwrap(), Token::Minus);
        assert_eq!(lexer.next_token().unwrap(), Token::Multiply);
        assert_eq!(lexer.next_token().unwrap(), Token::Divide);
    }

    #[test]
    fn test_literals() {
        let mut lexer = Lexer::new("123 'a' \"hello\"");
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(123));
        assert_eq!(lexer.next_token().unwrap(), Token::Character('a'));
        assert_eq!(
            lexer.next_token().unwrap(),
            Token::String("hello".to_string())
        );
    }
}
