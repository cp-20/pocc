use crate::{
    code_generator::error::CodeGeneratorError,
    ir_generator::{
        IRBinaryOp, IRFunction, IRModule, IRNode, IRNodeKind, IRUnaryOp, IRValue, IRVariable,
    },
    physical_register::PhysicalRegister,
    register_allocator::{RegisterAllocation, RegisterAllocationFunction},
    virtual_register::VirtualRegister,
};
use std::fmt::Write;

pub struct CodeGenerator {
    allocation: RegisterAllocation,
    current_allocation: Option<RegisterAllocationFunction>,
    current_function_name: String,
}

impl CodeGenerator {
    pub fn new(allocation: &RegisterAllocation) -> Self {
        CodeGenerator {
            allocation: allocation.clone(),
            current_allocation: None,
            current_function_name: String::new(),
        }
    }

    pub fn generate(&mut self, module: &IRModule) -> Result<String, CodeGeneratorError> {
        let mut code = String::new();
        code.push_str(&self.generate_header());
        for function in &module.functions {
            self.current_allocation = self.allocation.functions.get(&function.name).cloned();
            self.current_function_name = function.name.clone();
            if self.current_allocation.is_none() {
                return Err(CodeGeneratorError {
                    message: format!("Function {} not found in allocation", function.name),
                });
            }
            code.push_str(&self.generate_function(function)?);
        }
        code.push_str(&self.generate_global_variables(module));
        code.push_str(&self.generate_strings(module));
        code.push_str(&self.generate_footer());
        Ok(code)
    }

    fn generate_header(&self) -> String {
        ".text\n".to_string()
    }

    fn generate_footer(&self) -> String {
        "\t.section .note.GNU-stack, \"\", @progbits\n".to_string()
    }

    fn generate_global_variables(&self, module: &IRModule) -> String {
        if module.global_variables.is_empty() {
            return String::new();
        }

        let mut code = String::new();
        writeln!(code, "\t.bss").unwrap();
        for var in &module.global_variables {
            let size = var.ty.size();
            writeln!(code, "\t.type\t{}, @object", var.name).unwrap();
            writeln!(code, "\t.globl\t{}", var.name).unwrap();
            writeln!(code, "\t.p2align\t4").unwrap();
            writeln!(code, "{}:", var.name).unwrap();
            writeln!(code, "\t.zero\t{size}").unwrap();
            writeln!(code, "\t.size\t{}, {size}", var.name).unwrap();
        }
        code
    }

    fn generate_strings(&self, module: &IRModule) -> String {
        if module.strings.is_empty() {
            return String::new();
        }

        let mut code = String::new();
        writeln!(code, "\t.section	.rodata.str1.1, \"aMS\", @progbits, 1").unwrap();
        for string in &module.strings {
            let label = string.label();
            writeln!(code, "{label}:").unwrap();
            writeln!(code, "\t.asciz\t\"{}\"", string.value.escape_debug()).unwrap();
            writeln!(code, "\t.size\t{label}, {}", string.value.len() + 1).unwrap();
        }

        code
    }

    fn generate_function(&self, function: &IRFunction) -> Result<String, CodeGeneratorError> {
        let Some(function_allocation) = &self.current_allocation else {
            return Err(CodeGeneratorError {
                message: format!("Function {} not found in allocation", function.name),
            });
        };

        let mut code = String::new();
        writeln!(code, "\t.globl\t{}", function.name.clone()).unwrap();
        writeln!(code, "\t.p2align\t4").unwrap();
        writeln!(code, "\t.type\t{},@function", function.name.clone()).unwrap();
        writeln!(code, "{}:", function.name.clone()).unwrap();

        let spill_count = function_allocation.max_spilled_registers;

        if spill_count > 0 {
            writeln!(code, "\tpushq\t%rbp").unwrap();
            writeln!(code, "\tmovq\t%rsp, %rbp").unwrap();
            // FIXME: function parameters in the stack should be considered
            writeln!(code, "\tsubq\t${}, %rsp", spill_count * 8).unwrap();
        }

        for (i, reg) in function_allocation.used_callee_save.iter().enumerate() {
            let offset = (spill_count - i) * 8;
            writeln!(code, "\tmovq\t{}, -{}(%rbp)", reg.name(), offset).unwrap();
        }

        let moves: Vec<_> = function
            .parameter_regs
            .iter()
            .zip(function.parameters.iter())
            .map(|(param, reg)| (param.clone(), reg.clone()))
            .collect();
        code.push_str(&self.generate_parallel_move(&moves)?);

        for block in &function.body {
            writeln!(code, ".{}_{}:", function.name, block.id).unwrap();
            for node in &block.nodes {
                code.push_str(&self.generate_node(node)?);
            }
        }

        if function.name == "main" {
            writeln!(code, "\tmovq\t$0, %rax").unwrap();
        }
        writeln!(code, ".{}_end:", function.name).unwrap();

        for (i, reg) in function_allocation.used_callee_save.iter().enumerate() {
            let offset = (spill_count - i) * 8;
            writeln!(code, "\tmovq\t-{}(%rbp), {}", offset, reg.name()).unwrap();
        }

        if spill_count > 0 {
            writeln!(code, "\tleave").unwrap();
        }

        writeln!(code, "\tretq").unwrap();
        code.push('\n');

        Ok(code)
    }

    fn generate_node(&self, node: &IRNode) -> Result<String, CodeGeneratorError> {
        let mut code = String::new();
        match &node.kind {
            IRNodeKind::Assign { variable, value } => {
                let (value_name, value_code) = self.generate_value(value)?;
                let var_name = self.generate_variable(variable)?;

                code.push_str(&value_code);
                writeln!(code, "\tmovq\t{value_name}, {var_name}").unwrap();
            }
            IRNodeKind::AddressAssignment { address, value } => {
                let (value_name, value_code) = self.generate_value(value)?;
                let (address_name, address_code) = self.generate_value(address)?;

                code.push_str(&value_code);
                code.push_str(&address_code);
                writeln!(code, "\tmovq\t{value_name}, ({address_name})").unwrap();
            }
            IRNodeKind::Spill { reg, offset } => {
                let reg_name = self.get_register_name(reg)?;
                writeln!(code, "\tmovq\t{}, -{}(%rbp)", reg_name, offset * 8).unwrap();
            }
            IRNodeKind::Unspill { reg, offset } => {
                let reg_name = self.get_register_name(reg)?;
                writeln!(code, "\tmovq\t-{}(%rbp), {}", offset * 8, reg_name).unwrap();
            }
            IRNodeKind::Lea {
                base,
                index,
                scaler,
                result,
            } => {
                let (base_name, base_code) = self.generate_value(base)?;
                let index_name = self.get_register_name(index)?;
                let result_name = self.get_register_name(result)?;

                code.push_str(&base_code);
                writeln!(
                    code,
                    "\tleaq\t({base_name}, {index_name}, {scaler}), {result_name}"
                )
                .unwrap();
            }
            IRNodeKind::BinaryOp {
                op,
                left,
                right,
                result,
                ..
            } => {
                let (left_name, left_code) = self.generate_value(left)?;
                let (right_name, right_code) = self.generate_value(right)?;
                let result_reg = self.get_register(result)?;
                let result_name = self.get_register_name(result)?;
                let op_name = match op {
                    IRBinaryOp::Add => "addq",
                    IRBinaryOp::Sub => "subq",
                    IRBinaryOp::Mul => "imulq",
                    IRBinaryOp::Div => "idivq",
                    IRBinaryOp::BitAnd => "andq",
                    IRBinaryOp::BitOr => "orq",
                    IRBinaryOp::Eq => "cmpq",
                    IRBinaryOp::Less => "cmpq",
                };

                code.push_str(&left_code);
                code.push_str(&right_code);
                if op == &IRBinaryOp::Div {
                    // result_name must be %rax
                    assert_eq!(result_reg, PhysicalRegister::RAX);
                    if left_name != "%rax" {
                        writeln!(code, "\tmovq\t{left_name}, %rax").unwrap();
                    }
                    writeln!(code, "\tcqto").unwrap();
                    // FIXME: 定数で割ろうとすると死ぬ
                    writeln!(code, "\tidivq\t{right_name}").unwrap();
                } else if matches!(op, IRBinaryOp::Eq | IRBinaryOp::Less) {
                    if matches!(right, IRValue::Immediate(_)) {
                        writeln!(code, "\tcmpq\t{right_name}, {left_name}").unwrap();
                    } else {
                        writeln!(code, "\tcmpq\t{left_name}, {right_name}").unwrap();
                    }
                } else if left_name == result_name {
                    writeln!(code, "\t{op_name} {right_name}, {left_name}").unwrap();
                } else if right_name == result_name {
                    writeln!(code, "\t{op_name}\t{left_name}, {right_name}").unwrap();
                    if op == &IRBinaryOp::Sub {
                        writeln!(code, "\tnegq\t{right_name}").unwrap();
                    }
                } else {
                    writeln!(code, "\tmovq\t{left_name}, {result_name}").unwrap();
                    writeln!(code, "\t{op_name}\t{right_name}, {result_name}").unwrap();
                }

                if op == &IRBinaryOp::Eq || op == &IRBinaryOp::Less {
                    let op_name = match op {
                        IRBinaryOp::Eq => "sete",
                        IRBinaryOp::Less => {
                            if matches!(right, IRValue::Immediate(_)) {
                                "setl"
                            } else {
                                "setg"
                            }
                        }
                        _ => unreachable!(),
                    };
                    writeln!(code, "\t{}\t{}", op_name, result_reg.byte_name()).unwrap();
                    writeln!(
                        code,
                        "\tmovzbq\t{}, {}",
                        result_reg.byte_name(),
                        result_name
                    )
                    .unwrap();
                }
            }

            IRNodeKind::UnaryOp {
                op,
                operand,
                result,
            } => {
                let (operand_name, operand_code) = self.generate_value(operand)?;
                let result_name = self.get_register_name(result)?;

                code.push_str(&operand_code);

                match op {
                    IRUnaryOp::Neg | IRUnaryOp::Not => {
                        let op_name = match op {
                            IRUnaryOp::Neg => "negq",
                            IRUnaryOp::Not => "notq",
                            _ => unreachable!(),
                        };
                        writeln!(code, "\t{op_name}\t{operand_name}").unwrap();
                        if operand_name != result_name {
                            writeln!(code, "\tmovq\t{operand_name}, {result_name}").unwrap();
                        }
                    }
                    IRUnaryOp::Address => {
                        // FIXME: Address operation currently not supported in code generation
                        return Err(CodeGeneratorError {
                            message: "Address operation currently not supported in code generation"
                                .to_string(),
                        });
                    }
                    IRUnaryOp::Deref => {
                        writeln!(code, "\tmovq\t({operand_name}), {result_name}").unwrap();
                    }
                }
            }
            IRNodeKind::FunctionCall {
                name,
                arguments,
                argument_regs,
                ..
            } => {
                let name = match name {
                    IRVariable::Global(name) => {
                        let is_library_func = matches!(
                            name.as_str(),
                            "printf" | "malloc" | "random" | "srandom" | "atol" | "exit"
                        );
                        if is_library_func {
                            format!("{name}@PLT")
                        } else {
                            name.clone()
                        }
                    }
                    IRVariable::Register(reg) => self.get_register_name(reg)?,
                };

                for arg in arguments {
                    let (_, arg_code) = self.generate_value(arg)?;
                    code.push_str(&arg_code);
                }

                // move arguments to argument_regs in parallel
                let reg_moves = arguments
                    .iter()
                    .zip(argument_regs.iter())
                    .filter_map(|(arg, reg)| {
                        arg.get_register().map(|arg_reg| (arg_reg, reg.clone()))
                    })
                    .collect::<Vec<_>>();
                code.push_str(&self.generate_parallel_move(&reg_moves)?);

                let other_moves = arguments
                    .iter()
                    .zip(argument_regs.iter())
                    .map(|(arg, reg)| match arg {
                        IRValue::GlobalVariable(_) | IRValue::Immediate(_) => self
                            .generate_value(arg)
                            .map(|(arg_name, _)| Some((arg_name, reg.clone()))),
                        _ => Ok(None),
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                for (other_name, reg) in other_moves {
                    writeln!(
                        code,
                        "\tmovq\t{other_name}, {}",
                        self.get_register_name(&reg)?
                    )
                    .unwrap();
                }

                if name == "printf@PLT" {
                    writeln!(code, "\tmovb\t$0, %al").unwrap();
                }

                // NOTE: result are already set
                writeln!(code, "\tcallq\t{name}").unwrap();
            }
            IRNodeKind::Branch {
                condition,
                true_branch,
                false_branch,
            } => {
                let true_label =
                    true_branch.map(|b| format!(".{}_{}", self.current_function_name, b));
                let false_label =
                    false_branch.map(|b| format!(".{}_{}", self.current_function_name, b));

                if let Some(condition) = condition {
                    let (cond_value, cond_code) = self.generate_value(condition)?;
                    code.push_str(&cond_code);
                    writeln!(code, "\tcmpq\t$0, {cond_value}").unwrap();
                    if let Some(true_label) = true_label {
                        writeln!(code, "\tjne\t{true_label}").unwrap();
                    }
                    if let Some(false_label) = false_label {
                        writeln!(code, "\tje\t{false_label}").unwrap();
                    }
                } else {
                    if let Some(true_label) = true_label {
                        writeln!(code, "\tjmp\t{true_label}").unwrap();
                    }
                    if let Some(false_label) = false_label {
                        writeln!(code, "\tjmp\t{false_label}").unwrap();
                    }
                }
            }
            IRNodeKind::Return { value } => {
                if let Some(value) = value {
                    let (value_name, value_code) = self.generate_value(value)?;
                    code.push_str(&value_code);
                    writeln!(code, "\tmovq\t{value_name}, %rax").unwrap();
                }
                writeln!(code, "\tjmp\t.{}_end", self.current_function_name).unwrap();
            }
            _ => {}
        }

        Ok(code)
    }

    fn generate_value(&self, value: &IRValue) -> Result<(String, String), CodeGeneratorError> {
        match &value {
            IRValue::GlobalVariable(name) => Ok((format!("{name}(%rip)"), "".to_string())),
            IRValue::Immediate(value) => Ok((format!("${value}"), "".to_string())),
            IRValue::StringLiteral { string, reg } => Ok((
                self.get_register_name(reg)?,
                format!(
                    "\tleaq\t{}(%rip), {}\n",
                    string.label(),
                    self.get_register_name(reg)?
                ),
            )),
            IRValue::Register(reg) => Ok((self.get_register_name(reg)?, "".to_string())),
        }
    }

    fn generate_variable(&self, variable: &IRVariable) -> Result<String, CodeGeneratorError> {
        match variable {
            IRVariable::Global(name) => Ok(format!("{name}(%rip)")),
            IRVariable::Register(reg) => self.get_register_name(reg),
        }
    }

    fn generate_parallel_move(
        &self,
        moves: &[(VirtualRegister, VirtualRegister)],
    ) -> Result<String, CodeGeneratorError> {
        let mut code = String::new();

        let moves: Vec<(PhysicalRegister, PhysicalRegister)> = moves
            .iter()
            .map(|(src, dst)| {
                let src_physical = self.get_register(src)?;
                let dst_physical = self.get_register(dst)?;
                if src_physical == dst_physical {
                    Ok(None)
                } else {
                    Ok(Some((src_physical, dst_physical)))
                }
            })
            .collect::<Result<Vec<_>, CodeGeneratorError>>()?
            .into_iter()
            .flatten()
            .collect();

        let (chains, cycles) = {
            let mut chains: Vec<Vec<PhysicalRegister>> = Vec::new();
            let mut cycles: Vec<Vec<PhysicalRegister>> = Vec::new();
            for (src, dst) in moves {
                if src == dst {
                    continue;
                }
                if let Some(chain_index) = chains.iter().position(|c| c.last() == Some(&src)) {
                    let is_cycle = chains[chain_index].first() == Some(&dst);
                    chains[chain_index].push(dst);
                    if is_cycle {
                        cycles.push(chains[chain_index].clone());
                        chains.remove(chain_index);
                    }
                } else {
                    chains.push(vec![src, dst]);
                }
            }

            (chains, cycles)
        };

        for chain in chains {
            if chain.len() < 2 {
                continue;
            }
            let mut dst = chain.last().unwrap();
            for src in chain.iter().rev().skip(1) {
                writeln!(code, "\tmovq\t{}, {}", src.name(), dst.name()).unwrap();
                dst = src;
            }
        }

        for cycle in cycles {
            if cycle.len() < 2 {
                continue;
            }
            let mut dst = cycle.last().unwrap();
            for src in cycle.iter().rev().skip(1) {
                writeln!(code, "\txchgq\t{}, {}", src.name(), dst.name()).unwrap();
                dst = src;
            }
        }

        Ok(code)
    }

    fn get_register(&self, reg: &VirtualRegister) -> Result<PhysicalRegister, CodeGeneratorError> {
        let Some(allocation) = &self.current_allocation else {
            return Err(CodeGeneratorError {
                message: format!("No current allocation for register {}", reg.id),
            });
        };

        allocation.get(reg).cloned().ok_or(CodeGeneratorError {
            message: format!("Register {} not allocated", reg.id),
        })
    }

    fn get_register_name(&self, reg: &VirtualRegister) -> Result<String, CodeGeneratorError> {
        let physical_reg = self.get_register(reg)?;
        Ok(physical_reg.name().to_string())
    }
}
