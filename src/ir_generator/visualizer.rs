use crate::{
    ir_generator::{
        IRFunction,
        domain::{
            IRBinaryOp, IRBlock, IRModule, IRNode, IRNodeKind, IRString, IRUnaryOp, IRValue,
            IRVariable, VariableType,
        },
    },
    register_allocator::{RegisterAllocation, RegisterAllocationFunction},
    virtual_register::VirtualRegister,
};

use std::fmt::Write;

impl std::fmt::Display for IRModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_inner(&|_, reg| reg.to_string()))
    }
}

impl IRModule {
    pub fn fmt_with_allocation(&self, allocation: &RegisterAllocation) -> String {
        self.fmt_inner(&|name, reg| {
            allocation
                .functions
                .get(name)
                .and_then(|func_alloc| func_alloc.get(reg).map(|r| r.name().to_string()))
                .unwrap_or_else(|| reg.to_string())
        })
    }

    fn fmt_inner(&self, reg_mapping: &impl Fn(&String, &VirtualRegister) -> String) -> String {
        let mut output = String::new();
        for function in &self.functions {
            output.push_str(&function.fmt_inner(&|reg| reg_mapping(&function.name, reg)));
        }
        for var in &self.global_variables {
            writeln!(output, "Global Variable: {} ({})", var.name, var.ty).unwrap();
        }
        for string in &self.strings {
            writeln!(
                output,
                "String: {} = \"{}\"",
                string.id,
                string.value.escape_debug()
            )
            .unwrap();
        }
        output
    }
}

impl std::fmt::Display for IRFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_inner(&|reg| reg.to_string()))
    }
}

impl IRFunction {
    pub fn fmt_with_allocation(&self, allocation: &RegisterAllocationFunction) -> String {
        self.fmt_inner(&|reg| {
            allocation
                .get(reg)
                .map(|r| r.name().to_string())
                .unwrap_or_else(|| reg.to_string())
        })
    }

    fn fmt_inner(&self, reg_mapping: &impl Fn(&VirtualRegister) -> String) -> String {
        let mut output = String::new();
        let params = self
            .parameters
            .iter()
            .zip(&self.parameter_regs)
            .map(|(param, reg)| format!("{} <- {}", reg_mapping(param), reg_mapping(reg)))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(output, "Function: {} ({})", self.name, params).unwrap();
        for block in &self.body {
            output.push_str(&block.fmt_inner(reg_mapping));
        }
        output
    }
}

impl std::fmt::Display for IRBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "p{}:", self.id)?;
        for node in &self.nodes {
            writeln!(f, "  {node}")?;
        }
        Ok(())
    }
}

impl IRBlock {
    pub fn fmt_with_allocation(&self, allocation: &RegisterAllocationFunction) -> String {
        self.fmt_inner(&|reg| {
            allocation
                .get(reg)
                .map(|r| r.name().to_string())
                .unwrap_or_else(|| reg.to_string())
        })
    }

    fn fmt_inner(&self, reg_mapping: &impl Fn(&VirtualRegister) -> String) -> String {
        let mut output = String::new();
        writeln!(output, "p{}:", self.id).unwrap();
        for node in &self.nodes {
            writeln!(output, "  {}", node.fmt_inner(reg_mapping)).unwrap();
        }
        output
    }
}

impl std::fmt::Display for IRNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_inner(&|reg| reg.to_string()))
    }
}

impl IRNode {
    pub fn fmt_with_allocation(&self, allocation: &RegisterAllocationFunction) -> String {
        self.fmt_inner(&|reg| {
            allocation
                .get(reg)
                .map(|r| r.name().to_string())
                .unwrap_or_else(|| reg.to_string())
        })
    }

    fn fmt_inner<T: Fn(&VirtualRegister) -> String>(&self, reg_mapping: &T) -> String {
        match &self.kind {
            IRNodeKind::NoOp => "nop".to_string(),
            IRNodeKind::VariableDeclaration { reg } => format!("var {reg}"),
            IRNodeKind::Assign { variable, value } => format!(
                "{} = {}",
                variable.fmt_inner(reg_mapping),
                value.fmt_inner(reg_mapping)
            ),
            IRNodeKind::AddressAssignment { address, value } => {
                format!(
                    "({}) = {}",
                    address.fmt_inner(reg_mapping),
                    value.fmt_inner(reg_mapping)
                )
            }
            IRNodeKind::Lea {
                base,
                index,
                scaler,
                result,
            } => format!(
                "{} = {} + {} * {}",
                reg_mapping(result),
                base.fmt_inner(reg_mapping),
                reg_mapping(index),
                scaler
            ),
            IRNodeKind::Spill { reg, offset } => {
                format!("spill {} to {offset}", reg_mapping(reg))
            }
            IRNodeKind::Unspill { reg, offset } => {
                format!("unspill {} from {offset}", reg_mapping(reg))
            }
            IRNodeKind::BinaryOp {
                op,
                left,
                right,
                result,
                optional_result,
            } => format!(
                "{} = {} {} {}{}",
                reg_mapping(result),
                left.fmt_inner(reg_mapping),
                op,
                right.fmt_inner(reg_mapping),
                optional_result
                    .clone()
                    .map_or("".to_string(), |r| format!(" ({r})"))
            ),
            IRNodeKind::UnaryOp {
                op,
                operand,
                result,
            } => format!(
                "{} = {}{}",
                reg_mapping(result),
                op,
                operand.fmt_inner(reg_mapping)
            ),
            IRNodeKind::FunctionCall {
                name,
                arguments,
                result,
                argument_regs,
            } => {
                let arguments_str = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        format!("{} -> {}", arg.fmt_inner(reg_mapping), argument_regs[i])
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                if let Some(res) = result {
                    format!("{res} = call {name}({arguments_str})")
                } else {
                    format!("call {name}({arguments_str})")
                }
            }
            IRNodeKind::Branch {
                condition,
                true_branch,
                false_branch,
            } => {
                if let Some(cond) = condition {
                    format!(
                        "if {} goto {} else goto {}",
                        cond.fmt_inner(reg_mapping),
                        true_branch.map_or("none".to_string(), |t| format!("p{t}")),
                        false_branch.map_or("none".to_string(), |f| format!("p{f}"))
                    )
                } else {
                    format!(
                        "goto {}",
                        true_branch.map_or("none".to_string(), |t| format!("p{t}"))
                    )
                }
            }
            IRNodeKind::Return { value } => match value {
                Some(val) => format!("return {}", val.fmt_inner(reg_mapping)),
                None => "return".to_string(),
            },
        }
    }
}

impl std::fmt::Display for VariableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariableType::Char => write!(f, "char"),
            VariableType::Int => write!(f, "int"),
            VariableType::Long => write!(f, "long"),
            VariableType::Pointer => write!(f, "pointer"),
        }
    }
}

impl std::fmt::Display for IRValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_inner(&|reg| reg.to_string()))
    }
}

impl IRValue {
    pub fn fmt_inner<T: Fn(&VirtualRegister) -> String>(&self, reg_mapping: &T) -> String {
        match self {
            IRValue::Register(reg) => reg_mapping(reg),
            IRValue::Immediate(value) => value.to_string(),
            IRValue::GlobalVariable(name) => format!("#{name}"),
            IRValue::StringLiteral { string, reg } => {
                format!(
                    "\"{}\" <- {}",
                    string.value.escape_debug(),
                    reg_mapping(reg)
                )
            }
        }
    }
}

impl std::fmt::Display for IRVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRVariable::Register(reg) => write!(f, "%{}", reg.id),
            IRVariable::Global(name) => write!(f, "@{name}"),
        }
    }
}

impl IRVariable {
    pub fn fmt_inner<T: Fn(&VirtualRegister) -> String>(&self, reg_mapping: &T) -> String {
        match self {
            IRVariable::Register(reg) => reg_mapping(reg),
            IRVariable::Global(name) => format!("@{name}"),
        }
    }
}

impl std::fmt::Display for IRString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.value.escape_debug())
    }
}

impl std::fmt::Display for IRBinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRBinaryOp::Add => write!(f, "+"),
            IRBinaryOp::Sub => write!(f, "-"),
            IRBinaryOp::Mul => write!(f, "*"),
            IRBinaryOp::Div => write!(f, "/"),
            IRBinaryOp::Eq => write!(f, "=="),
            IRBinaryOp::Less => write!(f, "<"),
            IRBinaryOp::BitAnd => write!(f, "&"),
            IRBinaryOp::BitOr => write!(f, "|"),
        }
    }
}

impl std::fmt::Display for IRUnaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRUnaryOp::Neg => write!(f, "-"),
            IRUnaryOp::Not => write!(f, "!"),
            IRUnaryOp::Address => write!(f, "&"),
            IRUnaryOp::Deref => write!(f, "*"),
        }
    }
}
