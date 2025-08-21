#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeAnalyzerError {
    pub message: String,
}

impl LifetimeAnalyzerError {
    pub fn new(message: String) -> Self {
        LifetimeAnalyzerError { message }
    }
}

impl std::fmt::Display for LifetimeAnalyzerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LifetimeAnalyzerError: {}", self.message)
    }
}
