#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhysicalRegister {
    RAX,
    RBX,
    RCX,
    RDX,
    RDI,
    RSI,
    RSP,
    RBP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl PhysicalRegister {
    pub fn id(&self) -> u32 {
        match self {
            PhysicalRegister::RAX => 0,
            PhysicalRegister::RBX => 1,
            PhysicalRegister::RCX => 2,
            PhysicalRegister::RDX => 3,
            PhysicalRegister::RDI => 4,
            PhysicalRegister::RSI => 5,
            PhysicalRegister::RSP => 6,
            PhysicalRegister::RBP => 7,
            PhysicalRegister::R8 => 8,
            PhysicalRegister::R9 => 9,
            PhysicalRegister::R10 => 10,
            PhysicalRegister::R11 => 11,
            PhysicalRegister::R12 => 12,
            PhysicalRegister::R13 => 13,
            PhysicalRegister::R14 => 14,
            PhysicalRegister::R15 => 15,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            PhysicalRegister::RAX => "%rax",
            PhysicalRegister::RBX => "%rbx",
            PhysicalRegister::RCX => "%rcx",
            PhysicalRegister::RDX => "%rdx",
            PhysicalRegister::RDI => "%rdi",
            PhysicalRegister::RSI => "%rsi",
            PhysicalRegister::RSP => "%rsp",
            PhysicalRegister::RBP => "%rbp",
            PhysicalRegister::R8 => "%r8",
            PhysicalRegister::R9 => "%r9",
            PhysicalRegister::R10 => "%r10",
            PhysicalRegister::R11 => "%r11",
            PhysicalRegister::R12 => "%r12",
            PhysicalRegister::R13 => "%r13",
            PhysicalRegister::R14 => "%r14",
            PhysicalRegister::R15 => "%r15",
        }
    }

    pub fn dword_name(&self) -> &str {
        match self {
            PhysicalRegister::RAX => "%eax",
            PhysicalRegister::RBX => "%ebx",
            PhysicalRegister::RCX => "%ecx",
            PhysicalRegister::RDX => "%edx",
            PhysicalRegister::RDI => "%edi",
            PhysicalRegister::RSI => "%esi",
            PhysicalRegister::RSP => "%esp",
            PhysicalRegister::RBP => "%ebp",
            PhysicalRegister::R8 => "%r8d",
            PhysicalRegister::R9 => "%r9d",
            PhysicalRegister::R10 => "%r10d",
            PhysicalRegister::R11 => "%r11d",
            PhysicalRegister::R12 => "%r12d",
            PhysicalRegister::R13 => "%r13d",
            PhysicalRegister::R14 => "%r14d",
            PhysicalRegister::R15 => "%r15d",
        }
    }

    pub fn word_name(&self) -> &str {
        match self {
            PhysicalRegister::RAX => "%ax",
            PhysicalRegister::RBX => "%bx",
            PhysicalRegister::RCX => "%cx",
            PhysicalRegister::RDX => "%dx",
            PhysicalRegister::RDI => "%di",
            PhysicalRegister::RSI => "%si",
            PhysicalRegister::RSP => "%sp",
            PhysicalRegister::RBP => "%bp",
            PhysicalRegister::R8 => "%r8w",
            PhysicalRegister::R9 => "%r9w",
            PhysicalRegister::R10 => "%r10w",
            PhysicalRegister::R11 => "%r11w",
            PhysicalRegister::R12 => "%r12w",
            PhysicalRegister::R13 => "%r13w",
            PhysicalRegister::R14 => "%r14w",
            PhysicalRegister::R15 => "%r15w",
        }
    }

    pub fn byte_name(&self) -> &str {
        match self {
            PhysicalRegister::RAX => "%al",
            PhysicalRegister::RBX => "%bl",
            PhysicalRegister::RCX => "%cl",
            PhysicalRegister::RDX => "%dl",
            PhysicalRegister::RDI => "%dil",
            PhysicalRegister::RSI => "%sil",
            PhysicalRegister::RSP => "%spl",
            PhysicalRegister::RBP => "%bpl",
            PhysicalRegister::R8 => "%r8b",
            PhysicalRegister::R9 => "%r9b",
            PhysicalRegister::R10 => "%r10b",
            PhysicalRegister::R11 => "%r11b",
            PhysicalRegister::R12 => "%r12b",
            PhysicalRegister::R13 => "%r13b",
            PhysicalRegister::R14 => "%r14b",
            PhysicalRegister::R15 => "%r15b",
        }
    }

    pub fn is_caller_save(&self) -> bool {
        matches!(
            self,
            PhysicalRegister::RAX
                | PhysicalRegister::RCX
                | PhysicalRegister::RDX
                | PhysicalRegister::RDI
                | PhysicalRegister::RSI
                | PhysicalRegister::RSP
                | PhysicalRegister::R8
                | PhysicalRegister::R9
                | PhysicalRegister::R10
                | PhysicalRegister::R11
        )
    }

    pub fn is_callee_save(&self) -> bool {
        matches!(
            self,
            PhysicalRegister::RBX
                | PhysicalRegister::RBP
                | PhysicalRegister::R12
                | PhysicalRegister::R13
                | PhysicalRegister::R14
                | PhysicalRegister::R15
        )
    }
}

pub const ARGUMENT_REGISTERS: [PhysicalRegister; 6] = [
    PhysicalRegister::RDI,
    PhysicalRegister::RSI,
    PhysicalRegister::RDX,
    PhysicalRegister::RCX,
    PhysicalRegister::R8,
    PhysicalRegister::R9,
];

pub const CALLEE_SAVE_REGISTERS: [PhysicalRegister; 6] = [
    PhysicalRegister::RBX,
    PhysicalRegister::RBP,
    PhysicalRegister::R12,
    PhysicalRegister::R13,
    PhysicalRegister::R14,
    PhysicalRegister::R15,
];

pub const CALLER_SAVE_REGISTERS: [PhysicalRegister; 10] = [
    PhysicalRegister::RAX,
    PhysicalRegister::RCX,
    PhysicalRegister::RDX,
    PhysicalRegister::RDI,
    PhysicalRegister::RSI,
    PhysicalRegister::RSP,
    PhysicalRegister::R8,
    PhysicalRegister::R9,
    PhysicalRegister::R10,
    PhysicalRegister::R11,
];
