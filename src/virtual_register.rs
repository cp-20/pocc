use std::sync::atomic::{AtomicU32, Ordering};

use crate::physical_register::PhysicalRegister;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VirtualRegister {
    pub id: u32,
    pub stored: bool,
}

impl VirtualRegister {
    pub fn new(id: u32, stored: bool) -> Self {
        VirtualRegister { id, stored }
    }

    pub fn physical_dummy(physical: &PhysicalRegister) -> Self {
        VirtualRegister {
            id: 1000000 + physical.id(),
            stored: false,
        }
    }
}

impl std::fmt::Display for VirtualRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}", self.id)
    }
}

pub struct VirtualRegisterManager {
    counter: AtomicU32,
}

impl VirtualRegisterManager {
    pub fn new() -> Self {
        VirtualRegisterManager {
            counter: AtomicU32::new(0),
        }
    }

    pub fn allocate(&self, stored: bool) -> VirtualRegister {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        VirtualRegister::new(id, stored)
    }
}
