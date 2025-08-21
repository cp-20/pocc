use std::collections::{HashMap, HashSet};

use crate::virtual_register::VirtualRegister;

#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeTable {
    pub functions: Vec<FunctionLifetime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionLifetime {
    pub name: String,
    pub elements: Vec<LifetimeElement>,
    pub live_in: HashMap<IRAddress, HashSet<VirtualRegister>>,
    pub live_out: HashMap<IRAddress, HashSet<VirtualRegister>>,
}

impl FunctionLifetime {
    pub fn get_overlaps(&self) -> LifetimeOverlaps {
        let mut overlaps: HashSet<(VirtualRegister, VirtualRegister)> = HashSet::new();
        for regs_set in self.live_in.values() {
            let regs: Vec<_> = regs_set.iter().cloned().collect();
            for i in 0..regs.len() {
                for j in (i + 1)..regs.len() {
                    overlaps.insert((regs[i].clone(), regs[j].clone()));
                }
            }
        }

        LifetimeOverlaps::from_overlaps(overlaps)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeOverlaps {
    pub overlaps: HashSet<(VirtualRegister, VirtualRegister)>,
    pub overlaps_by_register: HashMap<VirtualRegister, HashSet<VirtualRegister>>,
}

impl LifetimeOverlaps {
    pub fn from_overlaps(overlaps: HashSet<(VirtualRegister, VirtualRegister)>) -> Self {
        let mut overlaps_by_register: HashMap<VirtualRegister, HashSet<VirtualRegister>> =
            HashMap::new();
        for (reg1, reg2) in &overlaps {
            overlaps_by_register
                .entry(reg1.clone())
                .or_default()
                .insert(reg2.clone());
            overlaps_by_register
                .entry(reg2.clone())
                .or_default()
                .insert(reg1.clone());
        }
        LifetimeOverlaps {
            overlaps,
            overlaps_by_register,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeElement {
    pub reg: VirtualRegister,
    pub references: Vec<IRAddress>,
    pub assigns: Vec<IRAddress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IRAddress {
    pub id: usize,
    pub offset: usize,
}

impl IRAddress {
    pub fn new(id: usize, offset: usize) -> Self {
        IRAddress { id, offset }
    }

    pub fn prev(&self) -> Self {
        IRAddress {
            id: self.id,
            offset: self.offset.saturating_sub(1),
        }
    }
}
