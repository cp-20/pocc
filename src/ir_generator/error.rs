use crate::ast::ASTNode;

#[derive(Debug, Clone, PartialEq)]
pub struct IRGeneratorError {
    pub message: String,
    pub node: Box<ASTNode>,
}

impl std::fmt::Display for IRGeneratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IRGeneratorError: {} at {}", self.message, self.node)
    }
}
