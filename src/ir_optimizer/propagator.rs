use std::collections::HashMap;

use crate::{
    ir_generator::{IRModule, IRNode, IRNodeKind, IRValue},
    ir_optimizer::{error::IROptimizerError, folder::fold_constants},
    virtual_register::VirtualRegister,
};

pub fn propagate_constants(module: &mut IRModule) -> Result<(), IROptimizerError> {
    let folded_registers = fold_constants(module)?;
    if folded_registers.is_empty() {
        return Ok(());
    }

    for function in &mut module.functions {
        for block in &mut function.body {
            for node in &mut block.nodes {
                replace_value(node, &folded_registers);
            }
        }
    }

    propagate_constants(module)
}

fn replace_value(node: &mut IRNode, folded: &HashMap<VirtualRegister, IRValue>) {
    if let IRNodeKind::Assign { variable, value } = &mut node.kind {
        node.kind = IRNodeKind::Assign {
            variable: variable.clone(),
            value: replace_with_folded(value, folded),
        };
    } else if let IRNodeKind::AddressAssignment { address, value } = &mut node.kind {
        node.kind = IRNodeKind::AddressAssignment {
            address: replace_with_folded(address, folded),
            value: replace_with_folded(value, folded),
        };
    } else if let IRNodeKind::Lea {
        base,
        index,
        scaler,
        result,
    } = &mut node.kind
    {
        node.kind = IRNodeKind::Lea {
            base: replace_with_folded(base, folded),
            index: index.clone(),
            scaler: *scaler,
            result: result.clone(),
        };
    } else if let IRNodeKind::BinaryOp {
        op,
        left,
        right,
        result,
        optional_result,
    } = &mut node.kind
    {
        node.kind = IRNodeKind::BinaryOp {
            op: op.clone(),
            left: replace_with_folded(left, folded),
            right: replace_with_folded(right, folded),
            result: result.clone(),
            optional_result: optional_result.clone(),
        };
    } else if let IRNodeKind::UnaryOp {
        op,
        operand,
        result,
    } = &mut node.kind
    {
        node.kind = IRNodeKind::UnaryOp {
            op: op.clone(),
            operand: replace_with_folded(operand, folded),
            result: result.clone(),
        };
    } else if let IRNodeKind::FunctionCall {
        name,
        arguments,
        argument_regs,
        result,
    } = &mut node.kind
    {
        node.kind = IRNodeKind::FunctionCall {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|arg| replace_with_folded(arg, folded))
                .collect(),
            argument_regs: argument_regs.clone(),
            result: result.clone(),
        };
    } else if let IRNodeKind::Branch {
        condition,
        true_branch,
        false_branch,
    } = &mut node.kind
    {
        node.kind = IRNodeKind::Branch {
            condition: condition.as_ref().map(|c| replace_with_folded(c, folded)),
            true_branch: *true_branch,
            false_branch: *false_branch,
        };
    } else if let IRNodeKind::Return { value } = &mut node.kind {
        node.kind = IRNodeKind::Return {
            value: value.as_ref().map(|v| replace_with_folded(v, folded)),
        };
    }
}

fn replace_with_folded(value: &IRValue, folded: &HashMap<VirtualRegister, IRValue>) -> IRValue {
    if let IRValue::Register(reg) = value
        && let Some(folded_value) = folded.get(reg)
    {
        folded_value.clone()
    } else {
        value.clone()
    }
}
