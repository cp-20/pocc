mod ast;
mod code_generator;
mod control_flow;
mod ir_generator;
mod ir_optimizer;
mod lexer;
mod lifetime_analyzer;
mod parser;
mod physical_register;
mod register_allocator;
mod symbol;
mod virtual_register;

use std::env;
use std::fs;
use std::process;

use crate::code_generator::CodeGenerator;
use crate::ir_generator::IRGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::register_allocator::RegisterAllocator;

const DELIMITER_LINE: &str =
    "===================================================================================";

struct MainError {
    message: String,
}

impl MainError {
    fn new(message: String) -> Self {
        MainError { message }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];

    match compile(filename) {
        Ok(assembly) => {
            println!("{assembly}");
        }
        Err(e) => {
            eprintln!("Error: {}", e.message);
            process::exit(1);
        }
    }
}

fn compile(filename: &str) -> Result<String, MainError> {
    let input = fs::read_to_string(filename)
        .map_err(|e| MainError::new(format!("Failed to read file '{filename}': {e}")))?;

    let lexer = Lexer::new(&input);
    let mut parser = Parser::new(lexer)
        .map_err(|e| MainError::new(format!("Parser initialization error: {e}")))?;

    let ast = parser
        .parse()
        .map_err(|e| MainError::new(format!("Parse error: {e}")))?;

    let mut ir_generator = IRGenerator::new();
    let ir_module = ir_generator
        .generate(&ast)
        .map_err(|e| MainError::new(format!("IR generation error: {e}")))?;

    let optimized_ir_module = ir_optimizer::optimize_ir(&ir_module)
        .map_err(|e| MainError::new(format!("IR optimization error: {e}")))?;

    eprintln!("{DELIMITER_LINE}");
    eprintln!("{}", optimized_ir_module);

    let lifetime_table = lifetime_analyzer::analyze_lifetime(&optimized_ir_module)
        .map_err(|e| MainError::new(format!("Lifetime analysis error: {e}")))?;

    eprintln!("{DELIMITER_LINE}");
    eprintln!("{}", lifetime_table);

    let register_allocator = RegisterAllocator::new(&lifetime_table);
    let (allocated_module, allocation) = register_allocator
        .allocate(&optimized_ir_module)
        .map_err(|e| MainError::new(format!("Register allocation error: {e}")))?;

    let mut code_generator = CodeGenerator::new(&allocation);
    let assembly = code_generator
        .generate(&allocated_module)
        .map_err(|e| MainError::new(format!("Code generation error: {e}")))?;

    Ok(assembly)
}
