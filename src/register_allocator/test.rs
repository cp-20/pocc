#[cfg(test)]
mod tests {
    use crate::{
        ir_generator::IRGenerator, lexer::Lexer, lifetime_analyzer::analyze_lifetime,
        parser::Parser, register_allocator::RegisterAllocator,
    };

    #[test]
    fn test_division() {
        // Arrange
        let input = "
void test_division(long a, long b) {
    long result;
    result = a / b;
}";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();
        let mut ir_generator = IRGenerator::new();
        let ir_module = ir_generator.generate(&ast).unwrap();
        let lifetime_table = analyze_lifetime(&ir_module).unwrap();

        // Act
        let register_allocator = RegisterAllocator::new(&lifetime_table);
        let (allocated_module, allocation) = register_allocator.allocate(&ir_module).unwrap();

        // Assert
        println!("{}", allocated_module.fmt_with_allocation(&allocation));
    }

    #[test]
    fn test_register_allocator() {
        // Arrange
        let input = "
void swap(long *a, long *b);

void bubble_sort(long *data, long size) {
    long i; long j;
    i = size - 1;
    while (0 < i) {
        j = 0;
        while (j < i) {
            if (*(data + (j+1)) < *(data + j))
                swap (data + j, data + (j + 1));
            j = j + 1;
        }
        i = i - 1;
    }
}";
        let lexer = Lexer::new(input);
        let mut parser = Parser::new(lexer).unwrap();
        let ast = parser.parse().unwrap();
        let mut ir_generator = IRGenerator::new();
        let ir_module = ir_generator.generate(&ast).unwrap();
        let lifetime_table = analyze_lifetime(&ir_module).unwrap();

        // Act
        let register_allocator = RegisterAllocator::new(&lifetime_table);
        let (allocated_module, allocation) = register_allocator.allocate(&ir_module).unwrap();

        // Assert
        println!("{}", allocated_module.fmt_with_allocation(&allocation));
    }
}
