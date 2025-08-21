use std::collections::{HashMap, HashSet};

use crate::{physical_register::PhysicalRegister, virtual_register::VirtualRegister};

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterAllocation {
    pub functions: HashMap<String, RegisterAllocationFunction>,
}

impl RegisterAllocation {
    pub fn new() -> Self {
        RegisterAllocation {
            functions: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisterAllocationFunction {
    pub mapping: HashMap<VirtualRegister, PhysicalRegister>,
    pub used_callee_save: HashSet<PhysicalRegister>,
    // NOTE: This includes both caller-save and callee-save registers
    pub max_spilled_registers: usize,
}

impl RegisterAllocationFunction {
    pub fn new() -> Self {
        RegisterAllocationFunction {
            mapping: HashMap::new(),
            used_callee_save: HashSet::new(),
            max_spilled_registers: 0,
        }
    }

    pub fn allocate(&mut self, virtual_reg: VirtualRegister, physical_reg: PhysicalRegister) {
        self.mapping.insert(virtual_reg, physical_reg);
    }

    pub fn get(&self, virtual_reg: &VirtualRegister) -> Option<&PhysicalRegister> {
        self.mapping.get(virtual_reg)
    }
}
