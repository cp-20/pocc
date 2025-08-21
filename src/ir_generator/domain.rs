use std::collections::HashMap;

use crate::virtual_register::VirtualRegister;

#[derive(Debug, Clone, PartialEq)]
pub struct IRModule {
    pub functions: Vec<IRFunction>,
    pub global_variables: Vec<IRVariableDeclaration>,
    pub strings: Vec<IRString>,
}

impl IRModule {
    pub fn compact(&mut self) {
        for function in &mut self.functions {
            function.compact();
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRVariableDeclaration {
    pub name: String,
    pub ty: VariableType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VariableType {
    Char,
    Int,
    Long,
    Pointer,
}

impl VariableType {
    pub fn size(&self) -> usize {
        match self {
            VariableType::Char => 1,
            VariableType::Int => 4,
            VariableType::Long => 8,
            VariableType::Pointer => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRFunction {
    pub name: String,
    // not bounded, free to allocate to any register
    pub parameters: Vec<VirtualRegister>,
    // forcibly bounded registers for parameters
    pub parameter_regs: Vec<VirtualRegister>,
    pub body: Vec<IRBlock>,
}

impl IRFunction {
    fn get_references(&self) -> Vec<usize> {
        self.body
            .iter()
            .flat_map(|block| block.nodes.iter())
            .flat_map(|node| match &node.kind {
                IRNodeKind::Branch {
                    true_branch,
                    false_branch,
                    ..
                } => [true_branch, false_branch]
                    .into_iter()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect()
    }

    fn update_block_ids(&mut self) {
        let mut id_mapping = HashMap::new();
        for (new_id, block) in self.body.iter().enumerate() {
            id_mapping.insert(block.id, new_id);
        }

        for block in &mut self.body {
            block.id = id_mapping[&block.id];
        }

        for block in &mut self.body {
            for node in &mut block.nodes {
                if let IRNodeKind::Branch {
                    true_branch,
                    false_branch,
                    ..
                } = &mut node.kind
                {
                    if let Some(tb) = true_branch {
                        *tb = id_mapping[tb];
                    }
                    if let Some(fb) = false_branch {
                        *fb = id_mapping[fb];
                    }
                }
            }
        }
    }

    pub fn compact(&mut self) {
        let references = self.get_references();

        // remove empty programs
        self.body
            .retain(|program| !program.nodes.is_empty() || references.contains(&program.id));

        // merge consecutive programs
        let mut merged_body: Vec<IRBlock> = Vec::new();
        for program in self.body.iter() {
            let is_referenced = references.contains(&program.id);
            if !is_referenced
                && let Some(last) = merged_body.last_mut()
                && !last.is_goto()
            {
                last.nodes.extend(program.nodes.clone());
            } else {
                merged_body.push(program.clone());
            }
        }

        self.body = merged_body;

        self.update_block_ids();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRBlock {
    pub id: usize,
    pub nodes: Vec<IRNode>,
}

impl IRBlock {
    pub fn is_goto(&self) -> bool {
        let last_node = self.nodes.last();
        last_node.map(|node| node.is_goto()).unwrap_or(false)
    }

    pub fn is_connected_to_next(&self) -> bool {
        let last_node = self.nodes.last();
        let Some(last_node) = last_node else {
            return true;
        };

        match &last_node.kind {
            IRNodeKind::Branch {
                true_branch: Some(_),
                false_branch: Some(_),
                ..
            }
            | IRNodeKind::Branch {
                condition: None, ..
            } => false,
            IRNodeKind::Branch { .. } => true,
            IRNodeKind::Return { .. } => false,
            _ => true,
        }
    }

    pub fn map_block_ids<F>(&mut self, f: F)
    where
        F: Fn(usize) -> usize,
    {
        self.id = f(self.id);
        for node in &mut self.nodes {
            if let IRNodeKind::Branch {
                true_branch,
                false_branch,
                ..
            } = &mut node.kind
            {
                if let Some(tb) = true_branch {
                    *tb = f(*tb);
                }
                if let Some(fb) = false_branch {
                    *fb = f(*fb);
                }
            }
        }
    }

    pub fn map_register_to_value<F>(&mut self, f: F)
    where
        F: Fn(VirtualRegister) -> IRValue,
    {
        let map_value = |value: IRValue| match value {
            IRValue::Register(reg) => f(reg),
            _ => value,
        };
        for node in &mut self.nodes {
            match &mut node.kind {
                IRNodeKind::Assign { value, .. } => {
                    *value = map_value(value.clone());
                }
                IRNodeKind::AddressAssignment { address, value } => {
                    *address = map_value(address.clone());
                    *value = map_value(value.clone());
                }
                IRNodeKind::Lea { base, .. } => {
                    *base = map_value(base.clone());
                }
                IRNodeKind::BinaryOp { left, right, .. } => {
                    *left = map_value(left.clone());
                    *right = map_value(right.clone());
                }
                IRNodeKind::UnaryOp { operand, .. } => {
                    *operand = map_value(operand.clone());
                }
                IRNodeKind::FunctionCall { arguments, .. } => {
                    for arg in arguments {
                        *arg = map_value(arg.clone());
                    }
                }
                IRNodeKind::Branch { condition, .. } => {
                    *condition = condition.as_ref().map(|cond| map_value(cond.clone()));
                }
                IRNodeKind::Return { value } => {
                    *value = value.as_ref().map(|val| map_value(val.clone()));
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRNode {
    pub kind: IRNodeKind,
}

impl IRNode {
    pub fn new(kind: IRNodeKind) -> Self {
        IRNode { kind }
    }

    pub fn is_goto(&self) -> bool {
        matches!(
            self.kind,
            IRNodeKind::Branch { .. } | IRNodeKind::Return { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRNodeKind {
    NoOp,
    VariableDeclaration {
        reg: VirtualRegister,
    },
    Assign {
        variable: IRVariable,
        value: IRValue,
    },
    AddressAssignment {
        address: IRValue,
        value: IRValue,
    },
    Spill {
        reg: VirtualRegister,
        // NOTE: offset is 0, 1, 2, ...
        offset: usize,
    },
    Unspill {
        reg: VirtualRegister,
        // NOTE: offset is 0, 1, 2, ...
        offset: usize,
    },
    Lea {
        base: IRValue,
        index: VirtualRegister,
        scaler: i64,
        result: VirtualRegister,
    },
    BinaryOp {
        op: IRBinaryOp,
        left: IRValue,
        right: IRValue,
        result: VirtualRegister,
        // NOTE: used for idiv op (%rdx is forcibly set)
        optional_result: Option<VirtualRegister>,
    },
    UnaryOp {
        op: IRUnaryOp,
        operand: IRValue,
        result: VirtualRegister,
    },
    FunctionCall {
        name: IRVariable,
        arguments: Vec<IRValue>,
        argument_regs: Vec<VirtualRegister>,
        result: Option<VirtualRegister>,
    },
    Branch {
        condition: Option<IRValue>,
        true_branch: Option<usize>,
        false_branch: Option<usize>,
    },
    Return {
        value: Option<IRValue>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRValue {
    GlobalVariable(String),
    Immediate(i64),
    StringLiteral {
        string: Box<IRString>,
        reg: VirtualRegister,
    },
    Register(VirtualRegister),
}

impl IRValue {
    pub fn is_constant(&self) -> bool {
        matches!(self, IRValue::Immediate(_) | IRValue::StringLiteral { .. })
    }
    pub fn get_immediate(&self) -> Option<i64> {
        match self {
            IRValue::Immediate(value) => Some(*value),
            _ => None,
        }
    }
}

impl IRValue {
    pub fn get_register(&self) -> Option<VirtualRegister> {
        match self {
            IRValue::Register(reg) => Some(reg.clone()),
            IRValue::StringLiteral { reg, .. } => Some(reg.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRString {
    pub id: usize,
    pub value: String,
}

impl IRString {
    pub fn label(&self) -> String {
        format!(".str.{}", self.id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Less,
    BitAnd,
    BitOr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRUnaryOp {
    Neg,
    Not,
    Address,
    Deref,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRVariable {
    Global(String),
    Register(VirtualRegister),
}

impl IRVariable {
    pub fn get_register(&self) -> Option<VirtualRegister> {
        match self {
            IRVariable::Global(_) => None,
            IRVariable::Register(reg) => Some(reg.clone()),
        }
    }
}
