#[cfg(test)]
mod tests {
    use crate::{
        ir_generator::IRGenerator, lexer::Lexer, lifetime_analyzer::analyzer::analyze_lifetime,
        parser::Parser,
    };

    #[test]
    fn test_lifetime_analyzer() {
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

        // Act
        let lifetime_table = analyze_lifetime(&ir_module).unwrap();

        // Assert
        let func = lifetime_table.functions[0].clone();
        assert_eq!(func.name, "bubble_sort");

        // FIXME: fragile test,
        assert_eq!(
            func.to_string(),
            "-------+----------------------+----------------------
Addr   | Live In              | Live Out            
-------+----------------------+----------------------
p0+0   |                      | %0, %1              
p0+2   | %0, %1               | %0, %4              
p0+3   | %0, %4               | %0, %2              
p1+0   | %0, %2               | %0, %2, %5          
p1+1   | %0, %2, %5           | %0, %2              
p2+0   | %0, %2               | %0, %2, %3          
p3+0   | %0, %2, %3           | %0, %2, %3, %6      
p3+1   | %0, %2, %3, %6       | %0, %2, %3          
p4+0   | %0, %2, %3           | %0, %2, %3, %7      
p4+1   | %0, %2, %3, %7       | %0, %2, %3, %9      
p4+2   | %0, %2, %3, %9       | %0, %2, %3, %8      
p4+3   | %0, %2, %3, %8       | %0, %10, %2, %3     
p4+4   | %0, %10, %2, %3      | %0, %10, %12, %2, %3
p4+5   | %0, %10, %12, %2, %3 | %0, %10, %11, %2, %3
p4+6   | %0, %10, %11, %2, %3 | %0, %10, %13, %2, %3
p4+7   | %0, %10, %13, %2, %3 | %0, %14, %2, %3     
p4+8   | %0, %14, %2, %3      | %0, %2, %3          
p5+0   | %0, %2, %3           | %0, %16, %2, %3     
p5+1   | %0, %16, %2, %3      | %0, %15, %2, %3     
p5+2   | %0, %15, %2, %3      | %0, %15, %17, %2, %3
p5+3   | %0, %15, %17, %2, %3 | %0, %15, %19, %2, %3
p5+4   | %0, %15, %19, %2, %3 | %0, %15, %18, %2, %3
p5+5   | %0, %15, %18, %2, %3 | %0, %2, %3          
p6+0   | %0, %2, %3           | %0, %2, %21         
p6+1   | %0, %2, %21          | %0, %2, %3          
p7+0   | %0, %2               | %0, %22             
p7+1   | %0, %22              | %0, %2              
"
        );
    }
}
