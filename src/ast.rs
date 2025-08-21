use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void,
    Bool,
    Char,
    Int,
    Long,
    Pointer(Box<Type>),
    Function {
        ret_type: Box<Type>,
        params: Vec<Parameter>,
    },
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Bool => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::Int => write!(f, "int"),
            Type::Long => write!(f, "long"),
            Type::Pointer(t) => write!(f, "*{t}"),
            Type::Function { ret_type, params } => {
                write!(f, "{ret_type} (")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Type {
    pub fn is_void(&self) -> bool {
        matches!(self, Type::Void)
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn is_function(&self) -> bool {
        matches!(self, Type::Function { .. })
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Type::Int | Type::Long | Type::Char | Type::Bool)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Bool)
    }

    pub fn deref(&self) -> Option<&Type> {
        if let Type::Pointer(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Bool => 1,
            Type::Char => 1,
            Type::Int => 4,
            Type::Long => 8,
            Type::Pointer(_) => 8,      // Assuming 64-bit architecture
            Type::Function { .. } => 8, // Function pointers are typically 8 bytes
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Address, // &
    Deref,   // *
    Plus,    // +
    Minus,   // -
    Not,     // !
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Address => write!(f, "&"),
            UnaryOp::Deref => write!(f, "*"),
            UnaryOp::Plus => write!(f, "+"),
            UnaryOp::Minus => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Assign, // =
    Or,     // ||
    And,    // &&
    Eq,     // ==
    Less,   // <
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Assign => write!(f, "="),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ASTNode {
    pub kind: ASTKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTKind {
    // Translation unit
    TranslationUnit(Vec<ASTNode>),

    // External declarations
    FunctionDefinition {
        ret_type: Type,
        name: String,
        params: Vec<Parameter>,
        body: Box<ASTNode>,
    },
    Declaration {
        var_type: Type,
        name: String,
    },

    // Statements
    ExpressionStatement(Option<Box<ASTNode>>),
    CompoundStatement {
        declarations: Vec<ASTNode>,
        statements: Vec<ASTNode>,
    },
    IfStatement {
        condition: Box<ASTNode>,
        then_stmt: Box<ASTNode>,
        else_stmt: Option<Box<ASTNode>>,
    },
    WhileStatement {
        condition: Box<ASTNode>,
        body: Box<ASTNode>,
    },
    ReturnStatement(Option<Box<ASTNode>>),

    // Expressions
    Identifier(String),
    IntegerLiteral(i64),
    CharacterLiteral(char),
    StringLiteral(String),
    BinaryExpression {
        op: BinaryOp,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    UnaryExpression {
        op: UnaryOp,
        operand: Box<ASTNode>,
    },
    FunctionCall {
        function: Box<ASTNode>,
        args: Vec<ASTNode>,
    },
    ParenExpression(Box<ASTNode>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub param_type: Type,
    pub name: String,
}

impl fmt::Display for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.param_type, self.name)
    }
}

impl ASTNode {
    pub fn new(kind: ASTKind) -> Self {
        ASTNode { kind }
    }
}

impl fmt::Display for ASTNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_with_indent(f, 0)
    }
}

impl ASTNode {
    fn display_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let spaces = "  ".repeat(indent);

        match &self.kind {
            ASTKind::TranslationUnit(nodes) => {
                writeln!(f, "{spaces}TranslationUnit")?;
                for node in nodes {
                    node.display_with_indent(f, indent + 1)?;
                }
            }
            ASTKind::FunctionDefinition {
                ret_type,
                name,
                params,
                body,
            } => {
                writeln!(f, "{spaces}FunctionDefinition: {ret_type} {name}")?;
                writeln!(f, "{spaces}  Parameters:")?;
                for param in params {
                    let param_type = &param.param_type;
                    let param_name = &param.name;
                    writeln!(f, "{spaces}    {param_type} {param_name}")?;
                }
                writeln!(f, "{spaces}  Body:")?;
                body.display_with_indent(f, indent + 2)?;
            }
            ASTKind::Declaration { var_type, name } => {
                writeln!(f, "{spaces}Declaration: {var_type} {name}")?;
            }
            ASTKind::CompoundStatement {
                declarations,
                statements,
            } => {
                writeln!(f, "{spaces}CompoundStatement")?;
                if !declarations.is_empty() {
                    writeln!(f, "{spaces}  Declarations:")?;
                    for decl in declarations {
                        decl.display_with_indent(f, indent + 2)?;
                    }
                }
                if !statements.is_empty() {
                    writeln!(f, "{spaces}  Statements:")?;
                    for stmt in statements {
                        stmt.display_with_indent(f, indent + 2)?;
                    }
                }
            }
            ASTKind::ExpressionStatement(expr) => {
                writeln!(f, "{spaces}ExpressionStatement")?;
                if let Some(expr) = expr {
                    expr.display_with_indent(f, indent + 1)?;
                }
            }
            ASTKind::IfStatement {
                condition,
                then_stmt,
                else_stmt,
            } => {
                writeln!(f, "{spaces}IfStatement")?;
                writeln!(f, "{spaces}  Condition:")?;
                condition.display_with_indent(f, indent + 2)?;
                writeln!(f, "{spaces}  Then:")?;
                then_stmt.display_with_indent(f, indent + 2)?;
                if let Some(else_stmt) = else_stmt {
                    writeln!(f, "{spaces}  Else:")?;
                    else_stmt.display_with_indent(f, indent + 2)?;
                }
            }
            ASTKind::WhileStatement { condition, body } => {
                writeln!(f, "{spaces}WhileStatement")?;
                writeln!(f, "{spaces}  Condition:")?;
                condition.display_with_indent(f, indent + 2)?;
                writeln!(f, "{spaces}  Body:")?;
                body.display_with_indent(f, indent + 2)?;
            }
            ASTKind::ReturnStatement(expr) => {
                writeln!(f, "{spaces}ReturnStatement")?;
                if let Some(expr) = expr {
                    expr.display_with_indent(f, indent + 1)?;
                }
            }
            ASTKind::Identifier(name) => {
                writeln!(f, "{spaces}Identifier: {name}")?;
            }
            ASTKind::IntegerLiteral(value) => {
                writeln!(f, "{spaces}IntegerLiteral: {value}")?;
            }
            ASTKind::CharacterLiteral(value) => {
                writeln!(f, "{spaces}CharacterLiteral: '{value}'")?;
            }
            ASTKind::StringLiteral(value) => {
                writeln!(f, "{spaces}StringLiteral: \"{}\"", value.escape_debug())?;
            }
            ASTKind::BinaryExpression { op, left, right } => {
                writeln!(f, "{spaces}BinaryExpression: {op:?}")?;
                writeln!(f, "{spaces}  Left:")?;
                left.display_with_indent(f, indent + 2)?;
                writeln!(f, "{spaces}  Right:")?;
                right.display_with_indent(f, indent + 2)?;
            }
            ASTKind::UnaryExpression { op, operand } => {
                writeln!(f, "{spaces}UnaryExpression: {op:?}")?;
                operand.display_with_indent(f, indent + 1)?;
            }
            ASTKind::FunctionCall { function, args } => {
                writeln!(f, "{spaces}FunctionCall")?;
                writeln!(f, "{spaces}  Function:")?;
                function.display_with_indent(f, indent + 2)?;
                if !args.is_empty() {
                    writeln!(f, "{spaces}  Arguments:")?;
                    for arg in args {
                        arg.display_with_indent(f, indent + 2)?;
                    }
                }
            }
            ASTKind::ParenExpression(expr) => {
                writeln!(f, "{spaces}ParenExpression")?;
                expr.display_with_indent(f, indent + 1)?;
            }
        }

        Ok(())
    }
}
