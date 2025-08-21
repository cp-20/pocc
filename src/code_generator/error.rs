use crate::ast::ASTNode;

#[derive(Debug, Clone, PartialEq)]
pub struct CodeGeneratorError {
    pub message: String,
}

impl std::fmt::Display for CodeGeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodeGeneratorError: {}", self.message)
    }
}
