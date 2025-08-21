use crate::{
    ir_generator::{IRModule, IRNodeKind, IRValue, IRVariable},
    ir_optimizer::error::IROptimizerError,
};

// transform assignments like:
//   %1 = ...
//   %2 = %1
// into:
//   %2 = ...
pub fn remove_redundant_assignments(module: &mut IRModule) -> Result<(), IROptimizerError> {
    for function in &mut module.functions {
        for block in &mut function.body {
            let mut nodes_to_remove = Vec::new();
            let len = block.nodes.len();
            let mut index = 0;
            while index + 1 < len {
                let (calc_node, assign_node) = {
                    let (left, right) = block.nodes.split_at_mut(index + 1);
                    (&mut left[index], &mut right[0])
                };

                let IRNodeKind::Assign {
                    variable: IRVariable::Register(assign_reg),
                    value: IRValue::Register(assign_value),
                } = &assign_node.kind
                else {
                    index += 1;
                    continue;
                };

                if let IRNodeKind::Assign {
                    variable: IRVariable::Register(calc_reg),
                    value: calc_value,
                } = &calc_node.kind
                {
                    if !calc_reg.stored && calc_reg == assign_value {
                        calc_node.kind = IRNodeKind::Assign {
                            variable: IRVariable::Register(assign_reg.clone()),
                            value: calc_value.clone(),
                        };
                        nodes_to_remove.push(index + 1);
                    }
                } else if let IRNodeKind::Lea {
                    base,
                    index: index_reg,
                    scaler,
                    result: calc_reg,
                } = &calc_node.kind
                {
                    if !calc_reg.stored && calc_reg == assign_value {
                        calc_node.kind = IRNodeKind::Lea {
                            base: base.clone(),
                            index: index_reg.clone(),
                            scaler: *scaler,
                            result: assign_reg.clone(),
                        };
                        nodes_to_remove.push(index + 1);
                    }
                } else if let IRNodeKind::BinaryOp {
                    op,
                    left,
                    right,
                    result,
                    optional_result,
                } = &calc_node.kind
                {
                    if !result.stored && result == assign_value {
                        calc_node.kind = IRNodeKind::BinaryOp {
                            op: op.clone(),
                            left: left.clone(),
                            right: right.clone(),
                            result: assign_reg.clone(),
                            optional_result: optional_result.clone(),
                        };
                        nodes_to_remove.push(index + 1);
                    }
                } else if let IRNodeKind::UnaryOp {
                    op,
                    operand,
                    result,
                } = &calc_node.kind
                {
                    if !result.stored && result == assign_value {
                        calc_node.kind = IRNodeKind::UnaryOp {
                            op: op.clone(),
                            operand: operand.clone(),
                            result: assign_reg.clone(),
                        };
                        nodes_to_remove.push(index + 1);
                    }
                } else if let IRNodeKind::FunctionCall {
                    name,
                    arguments,
                    argument_regs,
                    result,
                } = &calc_node.kind
                {
                    if let Some(result) = result
                        && !result.stored
                        && result == assign_value
                        // function call result will be always %rax, which can be different from assign_reg
                        && !assign_reg.stored
                    {
                        calc_node.kind = IRNodeKind::FunctionCall {
                            name: name.clone(),
                            arguments: arguments.clone(),
                            argument_regs: argument_regs.clone(),
                            result: Some(assign_reg.clone()),
                        };
                        nodes_to_remove.push(index + 1);
                    }
                }
                index += 1;
            }
            for &remove_index in nodes_to_remove.iter().rev() {
                block.nodes.remove(remove_index);
            }
        }
    }
    Ok(())
}
