#[cfg(test)]
mod tests {
    use crate::{
        ir_generator::{
            generator::IRGenerator,
            domain::{IRNode, IRNodeKind, IRValue},
        },
        lexer::Lexer,
        parser::Parser,
    };

    #[test]
    fn test_simple_return() {
        // Arrange
        let input = "int main() { return 42; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        // Act
        let mut generator = IRGenerator::new();
        let ir_module = generator.generate(&ast).unwrap();

        // Assert
        assert_eq!(ir_module.functions.len(), 1);
        assert_eq!(ir_module.functions[0].name, "main");
        assert_eq!(
            ir_module.functions[0].body[0].nodes[0],
            IRNode::new(IRNodeKind::Return {
                value: Some(IRValue::Immediate(42)),
            })
        );
    }

    #[test]
    fn test_variable_declaration() {
        // Arrange
        let input = "int main() { int x; return 0; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        // Act
        let mut generator = IRGenerator::new();
        let ir_module = generator.generate(&ast).unwrap();

        // Assert
        assert_eq!(ir_module.functions.len(), 1);
        let function = &ir_module.functions[0];
        assert_eq!(function.name, "main");
        assert!(matches!(
            function.body[0].nodes[0].kind,
            IRNodeKind::VariableDeclaration { .. }
        ));
    }

    #[test]
    fn test_assignment() {
        // Arrange
        let input = "int main() { int x; x = 42; return x; }";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();

        // Act
        let mut generator = IRGenerator::new();
        let ir_module = generator.generate(&ast).unwrap();

        // Assert
        assert_eq!(ir_module.functions.len(), 1);
        let function = &ir_module.functions[0];
        assert!(matches!(
            function.body[0].nodes[1].kind,
            IRNodeKind::Assign {
                value: IRValue::Immediate(42),
                ..
            }
        ));
    }
}
