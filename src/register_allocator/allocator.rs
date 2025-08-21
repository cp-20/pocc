use crate::{
    ir_generator::{IRFunction, IRModule, IRNode, IRNodeKind},
    lifetime_analyzer::LifetimeTable,
    register_allocator::{
        domain::{RegisterAllocation, RegisterAllocationFunction},
        error::RegisterAllocatorError,
        irc::allocate_registers,
    },
    virtual_register::VirtualRegister,
};

pub struct RegisterAllocator {
    lifetime_table: LifetimeTable,
}

impl RegisterAllocator {
    pub fn new(lifetime_table: &LifetimeTable) -> Self {
        RegisterAllocator {
            lifetime_table: lifetime_table.clone(),
        }
    }

    pub fn allocate(
        &self,
        module: &IRModule,
    ) -> Result<(IRModule, RegisterAllocation), RegisterAllocatorError> {
        let mut new_module = module.clone();
        new_module.functions.clear();
        let mut allocation = RegisterAllocation::new();

        for function in &module.functions {
            let (allocated_function, function_allocation) = self.allocate_function(function)?;
            new_module.functions.push(allocated_function);
            allocation
                .functions
                .insert(function.name.clone(), function_allocation);
        }

        Ok((new_module, allocation))
    }

    fn allocate_function(
        &self,
        function: &IRFunction,
    ) -> Result<(IRFunction, RegisterAllocationFunction), RegisterAllocatorError> {
        let lifetime = self
            .lifetime_table
            .functions
            .iter()
            .find(|f| f.name == function.name)
            .ok_or(RegisterAllocatorError::FunctionNotFound {
                function: function.name.clone(),
            })?;

        // let (allocation, spilled_regs) = self.graph_colorings.allocate(function, lifetime)?;

        // let mut new_function = function.clone();

        // Add spill code for registers that couldn't be allocated
        // if !spilled_regs.is_empty() {
        //     self.insert_spill_code(&mut new_function, &spilled_regs);
        // }

        let (new_function, allocation) = allocate_registers(function, lifetime)?;

        eprintln!("Allocated function {}:\n{}", function.name, allocation);

        eprintln!("{}", function.fmt_with_allocation(&allocation));

        Ok((new_function, allocation))
    }

    fn insert_spill_code(&self, function: &mut IRFunction, spilled_regs: &[VirtualRegister]) {
        let mut spill_offset = 0;
        let mut spill_map = std::collections::HashMap::new();

        // Assign stack offsets to spilled registers
        for reg in spilled_regs {
            spill_map.insert(reg.clone(), spill_offset);
            spill_offset += 1;
        }

        for block in &mut function.body {
            let mut new_nodes = Vec::new();

            for node in &block.nodes {
                // Insert unspill before use
                self.insert_unspill_before_use(&mut new_nodes, node, &spill_map);

                // Add the original node
                new_nodes.push(node.clone());

                // Insert spill after definition
                self.insert_spill_after_def(&mut new_nodes, node, &spill_map);
            }

            block.nodes = new_nodes;
        }
    }

    fn insert_unspill_before_use(
        &self,
        nodes: &mut Vec<IRNode>,
        node: &IRNode,
        spill_map: &std::collections::HashMap<VirtualRegister, usize>,
    ) {
        let used_regs = self.get_used_registers(node);
        for reg in used_regs {
            if let Some(&offset) = spill_map.get(&reg) {
                nodes.push(IRNode::new(IRNodeKind::Unspill { reg, offset }));
            }
        }
    }

    fn insert_spill_after_def(
        &self,
        nodes: &mut Vec<IRNode>,
        node: &IRNode,
        spill_map: &std::collections::HashMap<VirtualRegister, usize>,
    ) {
        let defined_regs = self.get_defined_registers(node);
        for reg in defined_regs {
            if let Some(&offset) = spill_map.get(&reg) {
                nodes.push(IRNode::new(IRNodeKind::Spill { reg, offset }));
            }
        }
    }

    fn get_used_registers(&self, node: &IRNode) -> Vec<VirtualRegister> {
        match &node.kind {
            IRNodeKind::Assign { value, .. } => value.get_register().into_iter().collect(),
            IRNodeKind::BinaryOp { left, right, .. } => [left.get_register(), right.get_register()]
                .into_iter()
                .flatten()
                .collect(),
            IRNodeKind::UnaryOp { operand, .. } => operand.get_register().into_iter().collect(),
            IRNodeKind::FunctionCall { arguments, .. } => arguments
                .iter()
                .filter_map(|arg| arg.get_register())
                .collect(),
            IRNodeKind::Branch { condition, .. } => condition
                .as_ref()
                .and_then(|c| c.get_register())
                .into_iter()
                .collect(),
            IRNodeKind::Return { value } => value
                .as_ref()
                .and_then(|v| v.get_register())
                .into_iter()
                .collect(),
            _ => vec![],
        }
    }

    fn get_defined_registers(&self, node: &IRNode) -> Vec<VirtualRegister> {
        match &node.kind {
            IRNodeKind::VariableDeclaration { reg } => vec![reg.clone()],
            IRNodeKind::BinaryOp { result, .. } => vec![result.clone()],
            IRNodeKind::UnaryOp { result, .. } => vec![result.clone()],
            IRNodeKind::Lea { result, .. } => vec![result.clone()],
            IRNodeKind::FunctionCall { result, .. } => {
                result.as_ref().into_iter().cloned().collect()
            }
            _ => vec![],
        }
    }
}
