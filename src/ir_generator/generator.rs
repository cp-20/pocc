use std::iter::once;

use crate::{
    ast::{ASTKind, ASTNode, BinaryOp, Type, UnaryOp},
    ir_generator::{
        domain::{
            IRBinaryOp, IRBlock, IRFunction, IRModule, IRNode, IRNodeKind, IRString, IRUnaryOp,
            IRValue, IRVariable, IRVariableDeclaration, VariableType,
        },
        error::IRGeneratorError,
    },
    symbol::{SymbolKind, SymbolTable},
    virtual_register::VirtualRegisterManager,
};

pub struct IRGenerator {
    virtual_register_manager: VirtualRegisterManager,
    strings: Vec<IRString>,
    symbol_table: SymbolTable,
    block_id: usize,
}

impl IRGenerator {
    pub fn new() -> Self {
        IRGenerator {
            virtual_register_manager: VirtualRegisterManager::new(),
            strings: Vec::new(),
            symbol_table: SymbolTable::new(),
            block_id: 0,
        }
    }

    pub fn generate(&mut self, ast: &ASTNode) -> Result<IRModule, IRGeneratorError> {
        let mut module = self.generate_root(ast)?;
        module.compact();
        Ok(module)
    }

    fn generate_root(&mut self, root: &ASTNode) -> Result<IRModule, IRGeneratorError> {
        let decls = {
            if let ASTKind::TranslationUnit(decls) = &root.kind {
                decls
            } else {
                return Err(IRGeneratorError {
                    message: "Expected TranslationUnit".to_string(),
                    node: Box::new(root.clone()),
                });
            }
        };

        let global_variables = decls
            .iter()
            .filter_map(|decl| {
                if let ASTKind::Declaration { var_type, name } = &decl.kind {
                    if let Err(err) = self
                        .symbol_table
                        .add_global_variable(name.clone(), var_type.clone())
                        .map_err(|e| IRGeneratorError {
                            message: format!("Failed to add global variable '{name}': {e}"),
                            node: Box::new(decl.clone()),
                        })
                    {
                        return Some(Err(err));
                    }

                    if var_type.is_function() {
                        return None;
                    }

                    Some(Ok(IRVariableDeclaration {
                        name: name.clone(),
                        ty: match var_type {
                            Type::Char => VariableType::Char,
                            Type::Int => VariableType::Int,
                            Type::Long => VariableType::Long,
                            Type::Pointer(_) => VariableType::Pointer,
                            _ => {
                                return Some(Err(IRGeneratorError {
                                    message: format!("Unsupported variable type: {var_type}"),
                                    node: Box::new(decl.clone()),
                                }));
                            }
                        },
                    }))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let functions = decls
            .iter()
            .filter_map(|decl| {
                if let ASTKind::FunctionDefinition { name, .. } = &decl.kind {
                    Some(self.generate_function(decl).map_err(|e| IRGeneratorError {
                        message: format!(
                            "Failed to generate function definition for '{}': {}",
                            name, e.message
                        ),
                        node: Box::new(decl.clone()),
                    }))
                } else {
                    None
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(IRModule {
            functions,
            global_variables,
            strings: self.strings.clone(),
        })
    }

    fn generate_function(&mut self, func: &ASTNode) -> Result<IRFunction, IRGeneratorError> {
        let (name, params, body, ret_type) = {
            if let ASTKind::FunctionDefinition {
                name,
                body,
                params,
                ret_type,
            } = &func.kind
            {
                (name, params, body, ret_type)
            } else {
                return Err(IRGeneratorError {
                    message: "Expected FunctionDefinition".to_string(),
                    node: Box::new(func.clone()),
                });
            }
        };

        self.symbol_table
            .add_function(name.clone(), ret_type.clone(), params.clone())
            .map_err(|e| IRGeneratorError {
                message: format!("Failed to add function '{name}': {e}"),
                node: Box::new(func.clone()),
            })?;

        self.symbol_table.enter_function(name.clone());
        self.symbol_table.enter_scope();

        let parameter_regs = params
            .iter()
            .map(|_| self.virtual_register_manager.allocate(false))
            .collect::<Vec<_>>();

        let parameters = params
            .iter()
            .map(|_| self.virtual_register_manager.allocate(true))
            .collect::<Vec<_>>();

        for (index, param) in params.iter().enumerate() {
            self.symbol_table
                .add_parameter(
                    param.name.clone(),
                    param.param_type.clone(),
                    parameters[index].clone(),
                )
                .map_err(|e| IRGeneratorError {
                    message: format!("Failed to add parameter '{}': {e}", param.name),
                    node: Box::new(func.clone()),
                })?;
        }

        // NOTE: This is for lifetime analysis
        let noop_block = self.new_block(vec![
            IRNode::new(IRNodeKind::NoOp),
            IRNode::new(IRNodeKind::NoOp),
        ]);
        let body_blocks = self.generate_stmt(body)?;
        let body = once(noop_block).chain(body_blocks).collect::<Vec<_>>();

        self.symbol_table.exit_function();
        self.symbol_table
            .exit_scope()
            .map_err(|e| IRGeneratorError {
                message: format!("Failed to exit scope: {e}"),
                node: Box::new(func.clone()),
            })?;

        Ok(IRFunction {
            name: name.clone(),
            parameters,
            parameter_regs,
            body,
        })
    }

    fn generate_stmt(&mut self, body: &ASTNode) -> Result<Vec<IRBlock>, IRGeneratorError> {
        match &body.kind {
            ASTKind::Declaration { name, var_type } => {
                let reg = self.virtual_register_manager.allocate(true);
                self.symbol_table
                    .add_local_variable(name.clone(), var_type.clone(), reg.clone())
                    .map_err(|e| IRGeneratorError {
                        message: format!("Failed to add local variable '{name}': {e}"),
                        node: Box::new(body.clone()),
                    })?;
                Ok(vec![self.new_block(vec![IRNode::new(
                    IRNodeKind::VariableDeclaration { reg },
                )])])
            }
            ASTKind::ExpressionStatement(expr) => {
                if let Some(expr) = expr {
                    let (blocks, _, _) = self.generate_expr(expr)?;
                    Ok(blocks)
                } else {
                    Ok(vec![self.new_block(vec![])])
                }
            }
            ASTKind::CompoundStatement {
                declarations,
                statements,
            } => {
                self.symbol_table.enter_scope();
                let decl_nodes = declarations
                    .iter()
                    .map(|decl| {
                        if let ASTKind::Declaration { name, var_type } = &decl.kind {
                            let reg = self.virtual_register_manager.allocate(true);
                            self.symbol_table
                                .add_local_variable(name.clone(), var_type.clone(), reg.clone())
                                .map_err(|e| IRGeneratorError {
                                    message: format!("Failed to add local variable '{name}': {e}"),
                                    node: Box::new(decl.clone()),
                                })?;
                            Ok(IRNode::new(IRNodeKind::VariableDeclaration { reg }))
                        } else {
                            Err(IRGeneratorError {
                                message: format!("Expected Declaration, found: {:?}", decl.kind),
                                node: Box::new(decl.clone()),
                            })
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let decl_block = self.new_block(decl_nodes);

                let body_blocks = statements
                    .iter()
                    .map(|stmt| self.generate_stmt(stmt))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                self.symbol_table
                    .exit_scope()
                    .map_err(|e| IRGeneratorError {
                        message: format!("Failed to exit scope: {e}"),
                        node: Box::new(body.clone()),
                    })?;

                let result_blocks = once(decl_block).chain(body_blocks).collect();

                Ok(result_blocks)
            }
            ASTKind::IfStatement {
                condition,
                then_stmt,
                else_stmt,
            } => {
                let (cond_blocks, _, cond_value) = self.generate_expr(condition)?;
                let then_blocks = self.generate_stmt(then_stmt)?;
                let end_block = self.new_block(vec![]);
                let result_blocks = if let Some(else_stmt) = else_stmt {
                    let else_blocks = self.generate_stmt(else_stmt)?;
                    let jump_block = self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                        condition: None,
                        true_branch: Some(end_block.id),
                        false_branch: None,
                    })]);
                    let block = self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                        condition: Some(cond_value.clone()),
                        true_branch: Some(then_blocks.first().unwrap().id),
                        false_branch: Some(else_blocks.first().unwrap().id),
                    })]);
                    cond_blocks
                        .into_iter()
                        .chain(once(block))
                        .chain(then_blocks)
                        .chain(once(jump_block))
                        .chain(else_blocks)
                        .chain(once(end_block))
                        .collect()
                } else {
                    let block = self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                        condition: Some(cond_value.clone()),
                        true_branch: Some(then_blocks.first().unwrap().id),
                        false_branch: Some(end_block.id),
                    })]);

                    cond_blocks
                        .into_iter()
                        .chain(once(block))
                        .chain(then_blocks)
                        .chain(once(end_block))
                        .collect()
                };

                Ok(result_blocks)
            }
            ASTKind::WhileStatement { condition, body } => {
                let (cond_blocks, _, cond_value) = self.generate_expr(condition)?;
                let body_block = self.generate_stmt(body)?;

                let while_end_block = self.new_block(vec![]);

                let while_start_block = self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                    condition: Some(cond_value.clone()),
                    true_branch: None,
                    false_branch: Some(while_end_block.id),
                })]);

                let body_end_block = self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                    condition: None,
                    true_branch: Some(cond_blocks.first().unwrap().id),
                    false_branch: None,
                })]);

                let result_blocks = cond_blocks
                    .into_iter()
                    .chain(once(while_start_block))
                    .chain(body_block)
                    .chain(once(body_end_block))
                    .chain(once(while_end_block))
                    .collect();

                Ok(result_blocks)
            }
            ASTKind::ReturnStatement(expr) => {
                if let Some(expr) = expr {
                    let (blocks, _, value) = self.generate_expr(expr)?;
                    let block = self
                        .new_block(vec![IRNode::new(IRNodeKind::Return { value: Some(value) })]);
                    let result_blocks = blocks.into_iter().chain(once(block)).collect();
                    Ok(result_blocks)
                } else {
                    Ok(vec![self.new_block(vec![IRNode::new(
                        IRNodeKind::Return { value: None },
                    )])])
                }
            }
            _ => Err(IRGeneratorError {
                message: format!("Unsupported AST node kind: {:?}", body.kind),
                node: Box::new(body.clone()),
            }),
        }
    }

    fn generate_expr(
        &mut self,
        expr: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        match &expr.kind {
            ASTKind::Identifier(name) => {
                let symbol = self.symbol_table.lookup(name).cloned();
                let Some(symbol) = symbol else {
                    return Err(IRGeneratorError {
                        message: format!("Undefined identifier: {name}"),
                        node: Box::new(expr.clone()),
                    });
                };

                let value = match &symbol.kind {
                    SymbolKind::LocalVariable { register, .. } => {
                        IRValue::Register(register.clone())
                    }
                    SymbolKind::Parameter { register, .. } => IRValue::Register(register.clone()),
                    SymbolKind::GlobalVariable => IRValue::GlobalVariable(name.clone()),
                    SymbolKind::Function { .. } => IRValue::GlobalVariable(name.clone()),
                };

                Ok((
                    vec![self.new_block(vec![])],
                    symbol.symbol_type.clone(),
                    value,
                ))
            }
            ASTKind::IntegerLiteral(value) => Ok((
                vec![self.new_block(vec![])],
                // FIXME: more precise type handling
                Type::Long,
                IRValue::Immediate(*value),
            )),
            ASTKind::CharacterLiteral(value) => Ok((
                vec![self.new_block(vec![])],
                // FIXME: more precise type handling
                Type::Long,
                IRValue::Immediate(*value as i64),
            )),
            ASTKind::StringLiteral(value) => {
                let ir_string = IRString {
                    id: self.strings.len(),
                    value: value.clone(),
                };
                self.strings.push(ir_string.clone());
                Ok((
                    // NOTE: This is for lifetime analysis
                    vec![self.new_block(vec![IRNode::new(IRNodeKind::NoOp)])],
                    Type::Pointer(Box::new(Type::Char)),
                    IRValue::StringLiteral {
                        string: Box::new(ir_string),
                        reg: self.virtual_register_manager.allocate(false),
                    },
                ))
            }
            ASTKind::BinaryExpression { op, left, right } => {
                self.generate_binary_op_expr(op, left, right)
            }
            ASTKind::UnaryExpression { op, operand } => self.generate_unary_op_expr(op, operand),
            ASTKind::FunctionCall { function, args } => {
                let (name, ret_type) = match &function.kind {
                    ASTKind::Identifier(name) => {
                        let symbol = self.symbol_table.lookup(name).cloned();
                        let Some(symbol) = symbol else {
                            return Err(IRGeneratorError {
                                message: format!("Undefined function: {name}"),
                                node: Box::new(*function.clone()),
                            });
                        };
                        let function_name = match &symbol.kind {
                            SymbolKind::Function { .. } | SymbolKind::GlobalVariable => {
                                IRVariable::Global(name.clone())
                            }
                            SymbolKind::LocalVariable { register, .. }
                            | SymbolKind::Parameter { register, .. } => {
                                IRVariable::Register(register.clone())
                            }
                        };

                        let ret_type = if let Type::Function { ret_type, .. } = &symbol.symbol_type
                        {
                            ret_type.clone()
                        } else {
                            return Err(IRGeneratorError {
                                message: format!("Expected function type for '{name}'"),
                                node: Box::new(*function.clone()),
                            });
                        };

                        Ok((function_name, ret_type))
                    }
                    _ => {
                        return Err(IRGeneratorError {
                            message: "Function call must be an identifier".to_string(),
                            node: Box::new(*function.clone()),
                        });
                    }
                }?;

                let arg_results: Vec<(Vec<IRBlock>, IRValue)> = args
                    .iter()
                    .map(|arg| {
                        let (blocks, _, value) = self.generate_expr(arg)?;
                        Ok((blocks, value))
                    })
                    .collect::<Result<Vec<_>, IRGeneratorError>>()?;

                let (arg_blocks, arg_values): (Vec<Vec<IRBlock>>, Vec<IRValue>) =
                    arg_results.into_iter().unzip();

                let arg_regs = arg_values
                    .iter()
                    .map(|_| self.virtual_register_manager.allocate(false))
                    .collect::<Vec<_>>();

                let result_register = self.virtual_register_manager.allocate(false);
                let block = self.new_block(vec![
                    // NOTE: This is for lifetime analysis
                    IRNode::new(IRNodeKind::NoOp),
                    IRNode::new(IRNodeKind::FunctionCall {
                        name: name.clone(),
                        arguments: arg_values.clone(),
                        argument_regs: arg_regs,
                        result: Some(result_register.clone()),
                    }),
                ]);

                let result_blocks = arg_blocks
                    .into_iter()
                    .flatten()
                    .chain(once(block))
                    .collect::<Vec<_>>();

                Ok((result_blocks, *ret_type, IRValue::Register(result_register)))
            }
            ASTKind::ParenExpression(inner) => self.generate_expr(inner),
            _ => Err(IRGeneratorError {
                message: format!("Unsupported AST node kind in expression: {:?}", expr.kind),
                node: Box::new(expr.clone()),
            }),
        }
    }

    fn generate_binary_op_expr(
        &mut self,
        op: &BinaryOp,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        match op {
            BinaryOp::Assign => self.generate_binary_op_assign(left, right),
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Eq
            | BinaryOp::Less => self.generate_binary_op_arithmetic(op, left, right),
            BinaryOp::And | BinaryOp::Or => self.generate_binary_op_logical(op, left, right),
        }
    }

    fn generate_binary_op_assign(
        &mut self,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        if let ASTKind::Identifier(name) = &left.kind {
            let symbol = self.symbol_table.lookup(name);
            let Some(symbol) = symbol else {
                return Err(IRGeneratorError {
                    message: format!("Undefined identifier: {name}"),
                    node: Box::new(left.clone()),
                });
            };

            let variable = match &symbol.kind {
                SymbolKind::LocalVariable { register, .. } => {
                    IRVariable::Register(register.clone())
                }
                SymbolKind::Parameter { register, .. } => IRVariable::Register(register.clone()),
                SymbolKind::GlobalVariable => IRVariable::Global(name.clone()),
                SymbolKind::Function { .. } => {
                    return Err(IRGeneratorError {
                        message: "Cannot assign to function".to_string(),
                        node: Box::new(left.clone()),
                    });
                }
            };

            let (right_blocks, right_type, right_value) = self.generate_expr(right)?;

            let block = self.new_block(vec![IRNode::new(IRNodeKind::Assign {
                variable,
                value: right_value.clone(),
            })]);

            let result_blocks = right_blocks.into_iter().chain(once(block)).collect();

            Ok((result_blocks, right_type, right_value))
        } else if let ASTKind::UnaryExpression { op, operand } = &left.kind
            && op == &UnaryOp::Deref
        {
            let (right_blocks, right_type, right_value) = self.generate_expr(right)?;
            let (operand_blocks, operand_type, operand_value) = self.generate_expr(operand)?;

            if !operand_type.is_pointer() {
                return Err(IRGeneratorError {
                    message: "Operand of the deref operator must be a pointer".to_string(),
                    node: Box::new(left.clone()),
                });
            }

            let block = self.new_block(vec![IRNode::new(IRNodeKind::AddressAssignment {
                address: operand_value,
                value: right_value.clone(),
            })]);

            let result_blocks = right_blocks
                .into_iter()
                .chain(operand_blocks)
                .chain(once(block))
                .collect();

            Ok((result_blocks, right_type, right_value))
        } else {
            Err(IRGeneratorError {
                message: "Left-hand side of assignment must be deref operator an identifier"
                    .to_string(),
                node: Box::new(left.clone()),
            })
        }
    }

    fn generate_binary_op_arithmetic(
        &mut self,
        op: &BinaryOp,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        let (left_blocks, left_type, left_value) = self.generate_expr(left)?;
        let (right_blocks, right_type, right_value) = self.generate_expr(right)?;

        let ir_op = match op {
            BinaryOp::Add => IRBinaryOp::Add,
            BinaryOp::Sub => IRBinaryOp::Sub,
            BinaryOp::Mul => IRBinaryOp::Mul,
            BinaryOp::Div => IRBinaryOp::Div,
            BinaryOp::Eq => IRBinaryOp::Eq,
            BinaryOp::Less => IRBinaryOp::Less,
            _ => unreachable!(),
        };

        let result_register = self.virtual_register_manager.allocate(false);
        let result_type =
            binary_op_type(op, &left_type, &right_type).map_err(|e| IRGeneratorError {
                message: e,
                node: Box::new(left.clone()),
            })?;

        // Handle pointer-number arithmetic: multiply numeric operand by 8
        let mut scale_to_pointer = |val: IRValue| {
            let scale_register = self.virtual_register_manager.allocate(false);
            let scale_block = self.new_block(vec![IRNode::new(IRNodeKind::BinaryOp {
                op: IRBinaryOp::Mul,
                left: val.clone(),
                right: IRValue::Immediate(8),
                result: scale_register.clone(),
                optional_result: None,
            })]);
            (IRValue::Register(scale_register), vec![scale_block])
        };

        let (final_left_value, final_right_value, additional_blocks) =
            match (op, &left_type, &right_type) {
                (BinaryOp::Add, left_type, right_type)
                    if left_type.is_pointer() && right_type.is_number() =>
                {
                    let (scaled_right, scale_blocks) = scale_to_pointer(right_value);
                    (left_value, scaled_right, scale_blocks)
                }
                (BinaryOp::Add, left_type, right_type)
                    if left_type.is_number() && right_type.is_pointer() =>
                {
                    let (scaled_left, scale_blocks) = scale_to_pointer(left_value);
                    (scaled_left, right_value, scale_blocks)
                }
                (BinaryOp::Sub, left_type, right_type)
                    if left_type.is_pointer() && right_type.is_number() =>
                {
                    let (scaled_right, scale_blocks) = scale_to_pointer(right_value);
                    (left_value, scaled_right, scale_blocks)
                }
                _ => (left_value, right_value, vec![]),
            };

        let optional_result_register = if op == &BinaryOp::Div {
            Some(self.virtual_register_manager.allocate(false))
        } else {
            None
        };

        let block = self.new_block(vec![IRNode::new(IRNodeKind::BinaryOp {
            op: ir_op,
            left: final_left_value,
            right: final_right_value,
            result: result_register.clone(),
            optional_result: optional_result_register,
        })]);

        let result_blocks = left_blocks
            .into_iter()
            .chain(right_blocks)
            .chain(additional_blocks)
            .chain(once(block))
            .collect();

        Ok((
            result_blocks,
            result_type,
            IRValue::Register(result_register),
        ))
    }

    fn generate_binary_op_logical(
        &mut self,
        op: &BinaryOp,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        let (left_blocks, left_type, left_value) = self.generate_expr(left)?;
        let (right_blocks, right_type, right_value) = self.generate_expr(right)?;

        let end_block = self.new_block(vec![]);
        let result_register = self.virtual_register_manager.allocate(true);
        let result_type =
            binary_op_type(op, &left_type, &right_type).map_err(|e| IRGeneratorError {
                message: e,
                node: Box::new(left.clone()),
            })?;

        let after_left_blocks = match op {
            BinaryOp::And => vec![
                self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                    condition: Some(left_value),
                    true_branch: Some(end_block.id),
                    false_branch: None,
                })]),
                self.new_block(vec![
                    IRNode::new(IRNodeKind::Assign {
                        variable: IRVariable::Register(result_register.clone()),
                        value: IRValue::Immediate(0),
                    }),
                    IRNode::new(IRNodeKind::Branch {
                        condition: None,
                        true_branch: Some(end_block.id),
                        false_branch: None,
                    }),
                ]),
            ],
            BinaryOp::Or => vec![
                self.new_block(vec![IRNode::new(IRNodeKind::Branch {
                    condition: Some(left_value),
                    true_branch: None,
                    false_branch: Some(right_blocks.first().unwrap().id),
                })]),
                self.new_block(vec![
                    IRNode::new(IRNodeKind::Assign {
                        variable: IRVariable::Register(result_register.clone()),
                        value: IRValue::Immediate(1),
                    }),
                    IRNode::new(IRNodeKind::Branch {
                        condition: None,
                        true_branch: Some(end_block.id),
                        false_branch: None,
                    }),
                ]),
            ],
            _ => unreachable!(),
        };

        let right_after_block = self.new_block(vec![
            IRNode::new(IRNodeKind::Assign {
                variable: IRVariable::Register(result_register.clone()),
                value: right_value.clone(),
            }),
            IRNode::new(IRNodeKind::BinaryOp {
                op: IRBinaryOp::BitAnd,
                left: IRValue::Register(result_register.clone()),
                right: IRValue::Immediate(1),
                result: result_register.clone(),
                optional_result: None,
            }),
        ]);

        let result_blocks = left_blocks
            .into_iter()
            .chain(after_left_blocks)
            .chain(right_blocks)
            .chain(once(right_after_block))
            .chain(once(end_block))
            .collect();

        Ok((
            result_blocks,
            result_type,
            IRValue::Register(result_register),
        ))
    }

    fn generate_unary_op_expr(
        &mut self,
        op: &UnaryOp,
        operand: &ASTNode,
    ) -> Result<(Vec<IRBlock>, Type, IRValue), IRGeneratorError> {
        let (operand_blocks, operand_type, operand_value) = self.generate_expr(operand)?;

        if op == &UnaryOp::Plus {
            return Ok((operand_blocks, operand_type, operand_value));
        }

        let result_register = self.virtual_register_manager.allocate(false);
        let result_type = unary_op_type(op, &operand_type).map_err(|e| IRGeneratorError {
            message: e,
            node: Box::new(operand.clone()),
        })?;

        let ir_op = match op {
            UnaryOp::Deref => IRUnaryOp::Deref,
            UnaryOp::Address => IRUnaryOp::Address,
            UnaryOp::Minus => IRUnaryOp::Neg,
            UnaryOp::Not => IRUnaryOp::Not,
            _ => unreachable!(),
        };

        let block = self.new_block(vec![IRNode::new(IRNodeKind::UnaryOp {
            op: ir_op,
            operand: operand_value.clone(),
            result: result_register.clone(),
        })]);

        let result_blocks = operand_blocks.into_iter().chain(once(block)).collect();

        Ok((
            result_blocks,
            result_type,
            IRValue::Register(result_register),
        ))
    }

    fn new_block(&mut self, nodes: Vec<IRNode>) -> IRBlock {
        self.block_id += 1;
        IRBlock {
            id: self.block_id,
            nodes,
        }
    }
}

fn binary_op_type(op: &BinaryOp, left: &Type, right: &Type) -> Result<Type, String> {
    match op {
        BinaryOp::Add => {
            if left.is_number() && right.is_number() {
                return Ok(Type::Long);
            } else if left.is_pointer() && right.is_number() {
                return Ok(left.clone());
            } else if left.is_number() && right.is_pointer() {
                return Ok(right.clone());
            }
        }
        BinaryOp::Sub => {
            if left.is_number() && right.is_number() {
                return Ok(Type::Long);
            } else if left.is_pointer() && right.is_number() {
                return Ok(left.clone());
            } else if left == right {
                return Ok(Type::Long);
            }
        }
        BinaryOp::Mul | BinaryOp::Div => {
            if left.is_number() && right.is_number() {
                return Ok(Type::Long);
            }
        }
        BinaryOp::And | BinaryOp::Or => {
            if (left.is_bool() || left.is_number()) && (right.is_bool() || right.is_number()) {
                return Ok(Type::Bool);
            }
        }
        BinaryOp::Eq => {
            if left.is_number() && right.is_number() || left == right {
                return Ok(Type::Bool);
            }
        }
        BinaryOp::Less => {
            if left.is_number() && right.is_number() {
                return Ok(Type::Bool);
            }
        }
        BinaryOp::Assign => {
            if left.is_number() && right.is_number() {
                return Ok(Type::Long);
            } else if left.is_pointer() && right.is_pointer() || left == right {
                // accept if both are pointers (for void* assignment)
                return Ok(left.clone());
            }
        }
    }

    Err(format!("Invalid binary op: {left} {op} {right}"))
}

fn unary_op_type(op: &UnaryOp, operand: &Type) -> Result<Type, String> {
    match op {
        UnaryOp::Plus => {
            if operand.is_number() {
                return Ok(Type::Long);
            }
        }
        UnaryOp::Minus => {
            if operand.is_number() {
                return Ok(Type::Long);
            }
        }
        UnaryOp::Not => {
            if operand.is_bool() || operand.is_number() {
                return Ok(Type::Bool);
            }
        }
        UnaryOp::Deref => {
            if let Type::Pointer(inner_type) = operand {
                return Ok(*inner_type.clone());
            }
        }
        UnaryOp::Address => return Ok(Type::Pointer(Box::new(operand.clone()))),
    }

    Err(format!("Invalid unary op: {op} {operand}"))
}
