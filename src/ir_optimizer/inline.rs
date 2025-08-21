use std::iter::once;

use crate::{
    ir_generator::{IRBlock, IRFunction, IRModule, IRNode, IRNodeKind, IRValue, IRVariable},
    ir_optimizer::error::IROptimizerError,
    virtual_register::VirtualRegister,
};

pub fn inline_functions(module: &mut IRModule) -> Result<bool, IROptimizerError> {
    let target_functions: Vec<_> = module
        .functions
        .iter()
        .filter(|function| can_inline(function))
        .cloned()
        .collect();

    if target_functions.is_empty() {
        return Ok(false);
    }

    for target in &target_functions {
        inline_function(module, target)?;
    }

    Ok(true)
}

fn can_inline(function: &IRFunction) -> bool {
    let num_nodes = function.body.iter().flat_map(|block| &block.nodes).count();
    let has_function_call = function.body.iter().any(|block| {
        block
            .nodes
            .iter()
            .any(|node| matches!(node.kind, IRNodeKind::FunctionCall { .. }))
    });
    let has_parameter_modification = function.body.iter().any(|block| {
        block.nodes.iter().any(|node| {
            if let IRNodeKind::Assign { variable, .. } = &node.kind
                && let IRVariable::Register(reg) = variable
            {
                return function.parameters.iter().any(|param| param == reg);
            }

            false
        })
    });
    num_nodes < 10 && !has_function_call && !has_parameter_modification
}

fn inline_function(module: &mut IRModule, target: &IRFunction) -> Result<(), IROptimizerError> {
    for function in &mut module.functions {
        if function.name == target.name {
            continue;
        }

        let mut block_offset = function.body.len();
        let mut get_block_offset = || {
            let offset = block_offset;
            block_offset += 1;
            offset
        };

        let mut block_index = 0;
        while block_index < function.body.len() {
            let block = &mut function.body[block_index];
            for node_index in 0..block.nodes.len() {
                let node = block.nodes[node_index].clone();
                if let IRNodeKind::FunctionCall {
                    name,
                    arguments,
                    result,
                    ..
                } = node.kind
                {
                    if let IRVariable::Global(name) = name
                        && *name == target.name
                    {
                        let before_call_nodes = block.nodes[..node_index].to_vec();
                        let before_call_block = IRBlock {
                            id: block.id,
                            nodes: before_call_nodes,
                        };
                        let after_call_nodes = block.nodes[node_index + 1..].to_vec();
                        let after_call_block = IRBlock {
                            id: get_block_offset(),
                            nodes: after_call_nodes,
                        };

                        let mut inlined_blocks = target.body.clone();

                        // re-mapping block ids
                        let block_id_mapping = inlined_blocks
                            .iter()
                            .map(|block| (block.id, get_block_offset()))
                            .collect::<std::collections::HashMap<_, _>>();
                        let map_block_id =
                            |id: usize| block_id_mapping.get(&id).cloned().unwrap_or(id);
                        for block in &mut inlined_blocks {
                            block.map_block_ids(map_block_id);
                        }

                        // re-mapping argument registers
                        let arg_reg_mapping = target
                            .parameters
                            .iter()
                            .zip(arguments.iter())
                            .map(|(func, call)| (func.clone(), call.clone()))
                            .collect::<std::collections::HashMap<_, _>>();
                        let map_register_to_value = |reg: VirtualRegister| {
                            arg_reg_mapping
                                .get(&reg)
                                .cloned()
                                .unwrap_or(IRValue::Register(reg))
                        };
                        for block in &mut inlined_blocks {
                            block.map_register_to_value(map_register_to_value);
                        }

                        // re-mapping result registers
                        let return_block = IRBlock {
                            id: get_block_offset(),
                            nodes: vec![],
                        };
                        for block in &mut inlined_blocks {
                            let mut inserting_nodes: Vec<(usize, IRNode)> = Vec::new();
                            for index in 0..block.nodes.len() {
                                let node = &mut block.nodes[index];
                                if let IRNodeKind::Return { value } = &mut node.kind {
                                    if let Some(ref result) = result
                                        && let Some(value) = value
                                    {
                                        inserting_nodes.push((
                                            index,
                                            IRNode {
                                                kind: IRNodeKind::Assign {
                                                    variable: IRVariable::Register(result.clone()),
                                                    value: value.clone(),
                                                },
                                            },
                                        ));
                                    }
                                    node.kind = IRNodeKind::Branch {
                                        condition: None,
                                        true_branch: Some(return_block.id),
                                        false_branch: None,
                                    }
                                }
                            }
                            for (index, node) in inserting_nodes.iter().rev() {
                                block.nodes.insert(*index, node.clone());
                            }
                        }

                        function.body.splice(
                            block_index..=block_index,
                            once(before_call_block)
                                .chain(inlined_blocks)
                                .chain(once(return_block))
                                .chain(once(after_call_block)),
                        );
                        block_index += target.body.len() + 1;
                        break;
                    }
                }
            }

            block_index += 1;
        }

        function.compact();
    }

    Ok(())
}
