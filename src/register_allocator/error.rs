#[derive(Debug, Clone, PartialEq)]
pub enum RegisterAllocatorError {
    FunctionNotFound { function: String },
    TooManyParameters { function: String, actual: usize },
    TooManyArguments { function: String, actual: usize },
    Other { message: String },
}

impl std::fmt::Display for RegisterAllocatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterAllocatorError::FunctionNotFound { function } => {
                write!(f, "Function '{function}' not found")
            }
            RegisterAllocatorError::TooManyParameters { function, actual } => {
                write!(
                    f,
                    "Too many parameters for function '{}': expected at most {}, got {}",
                    function, 6, actual
                )
            }
            RegisterAllocatorError::TooManyArguments { function, actual } => {
                write!(
                    f,
                    "Too many arguments for function '{}': expected at most {}, got {}",
                    function, 6, actual
                )
            }
            RegisterAllocatorError::Other { message } => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for RegisterAllocatorError {}
