use std::collections::HashMap;

use crate::{
    ir_generator::{IRBinaryOp, IRModule, IRNodeKind, IRUnaryOp, IRValue, IRVariable},
    ir_optimizer::error::IROptimizerError,
    virtual_register::VirtualRegister,
};

pub fn fold_constants(
    module: &mut IRModule,
) -> Result<HashMap<VirtualRegister, IRValue>, IROptimizerError> {
    let mut folded_registers = Vec::new();
    for function in &mut module.functions {
        for block in &mut function.body {
            let mut nodes_to_remove = Vec::new();
            for (index, node) in block.nodes.iter_mut().enumerate() {
                if let IRNodeKind::BinaryOp {
                    op,
                    left,
                    right,
                    result,
                    ..
                } = node.kind.clone()
                {
                    let folded_value = fold_binary_op(&op, &left, &right)?;
                    if let Some(value) = folded_value {
                        node.kind = IRNodeKind::Assign {
                            variable: IRVariable::Register(result.clone()),
                            value: value.clone(),
                        };
                        if !result.stored {
                            folded_registers.push((result.clone(), value));
                            nodes_to_remove.push(index);
                        }
                    }
                } else if let IRNodeKind::UnaryOp {
                    op,
                    operand,
                    result,
                } = node.kind.clone()
                {
                    let folded_value = fold_unary_op(&op, &operand)?;
                    if let Some(value) = folded_value {
                        node.kind = IRNodeKind::Assign {
                            variable: IRVariable::Register(result.clone()),
                            value: value.clone(),
                        };
                        if !result.stored {
                            folded_registers.push((result.clone(), value));
                            nodes_to_remove.push(index);
                        }
                    }
                } else if let IRNodeKind::Assign { variable, value } = node.kind.clone() {
                    if let IRVariable::Register(reg) = variable
                        && !reg.stored
                        && value.is_constant()
                    {
                        folded_registers.push((reg.clone(), value.clone()));
                        nodes_to_remove.push(index);
                    }
                } else if let IRNodeKind::Branch {
                    condition,
                    true_branch,
                    false_branch,
                } = node.kind.clone()
                {
                    if let Some(cond_value) = condition.and_then(|c| c.get_immediate()) {
                        if cond_value != 0 {
                            node.kind = IRNodeKind::Branch {
                                condition: None,
                                true_branch,
                                false_branch: None,
                            };
                        } else {
                            node.kind = IRNodeKind::Branch {
                                condition: None,
                                true_branch: false_branch,
                                false_branch: None,
                            };
                        }
                    }
                }
            }

            // Remove nodes in reverse order to maintain correct indices
            for &index in nodes_to_remove.iter().rev() {
                block.nodes.remove(index);
            }
        }
    }
    let folded_registers: HashMap<VirtualRegister, IRValue> =
        folded_registers.into_iter().collect();
    Ok(folded_registers)
}

fn fold_binary_op(
    op: &IRBinaryOp,
    left: &IRValue,
    right: &IRValue,
) -> Result<Option<IRValue>, IROptimizerError> {
    let Some(left_value) = left.get_immediate() else {
        return Ok(None);
    };
    let Some(right_value) = right.get_immediate() else {
        return Ok(None);
    };
    match op {
        IRBinaryOp::Add => Ok(Some(IRValue::Immediate(left_value + right_value))),
        IRBinaryOp::Sub => Ok(Some(IRValue::Immediate(left_value - right_value))),
        IRBinaryOp::Mul => Ok(Some(IRValue::Immediate(left_value * right_value))),
        IRBinaryOp::Div => {
            if right_value == 0 {
                Err(IROptimizerError {
                    message: "Division by zero".to_string(),
                })
            } else {
                Ok(Some(IRValue::Immediate(left_value / right_value)))
            }
        }
        IRBinaryOp::Eq => Ok(Some(IRValue::Immediate((left_value == right_value) as i64))),
        IRBinaryOp::Less => Ok(Some(IRValue::Immediate((left_value < right_value) as i64))),
        IRBinaryOp::BitAnd => Ok(Some(IRValue::Immediate(left_value & right_value))),
        IRBinaryOp::BitOr => Ok(Some(IRValue::Immediate(left_value | right_value))),
    }
}

fn fold_unary_op(op: &IRUnaryOp, operand: &IRValue) -> Result<Option<IRValue>, IROptimizerError> {
    let Some(value) = operand.get_immediate() else {
        return Ok(None);
    };
    match op {
        IRUnaryOp::Neg => Ok(Some(IRValue::Immediate(-value))),
        IRUnaryOp::Not => Ok(Some(IRValue::Immediate(!value))),
        _ => Ok(None),
    }
}
