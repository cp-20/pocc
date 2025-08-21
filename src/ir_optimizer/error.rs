#[derive(Debug, Clone, PartialEq)]
pub struct IROptimizerError {
    pub message: String,
}

impl std::fmt::Display for IROptimizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IROptimizerError: {}", self.message)
    }
}
