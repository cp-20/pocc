use crate::{
    ast::{Parameter, Type},
    virtual_register::VirtualRegister,
};
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone)]
pub enum SymbolKind {
    LocalVariable {
        offset: i32,
        register: VirtualRegister,
    },
    Parameter {
        offset: i32,
        register: VirtualRegister,
    },
    GlobalVariable,
    Function {
        params: Vec<Parameter>,
    },
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub symbol_type: Type,
    pub kind: SymbolKind,
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?} ({:?})", self.name, self.symbol_type, self.kind)
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub scopes: Vec<HashMap<String, Symbol>>,
    current_function: Option<String>,
    local_offset: i32,
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for scope in &self.scopes {
            for symbol in scope.values() {
                writeln!(f, "{symbol}")?;
            }
        }
        Ok(())
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            scopes: vec![HashMap::new()], // Global scope
            current_function: None,
            local_offset: 0,
        }
    }

    pub fn is_global_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) -> Result<(), String> {
        if self.scopes.len() == 1 {
            return Err("Cannot exit global scope".to_string());
        }

        self.scopes.pop().unwrap();
        Ok(())
    }

    pub fn enter_function(&mut self, name: String) {
        self.current_function = Some(name);
        self.local_offset = 0;
    }

    pub fn exit_function(&mut self) {
        self.current_function = None;
        self.local_offset = 0;
    }

    fn add_symbol(
        &mut self,
        name: String,
        symbol_type: Type,
        kind: SymbolKind,
    ) -> Result<(), String> {
        let current_scope = self.scopes.last_mut().unwrap();

        if current_scope.contains_key(&name) {
            return Err(format!("Symbol '{name}' already defined in current scope"));
        }

        let symbol = Symbol {
            name: name.clone(),
            symbol_type,
            kind,
        };

        current_scope.insert(name, symbol);
        Ok(())
    }

    pub fn add_local_variable(
        &mut self,
        name: String,
        symbol_type: Type,
        register: VirtualRegister,
    ) -> Result<i32, String> {
        self.local_offset -= 8; // Assume all variables are 8 bytes
        let offset = self.local_offset;

        self.add_symbol(
            name,
            symbol_type,
            SymbolKind::LocalVariable { offset, register },
        )?;
        Ok(offset)
    }

    pub fn add_parameter(
        &mut self,
        name: String,
        symbol_type: Type,
        register: VirtualRegister,
    ) -> Result<i32, String> {
        self.local_offset -= 8;
        let offset = self.local_offset;

        self.add_symbol(
            name,
            symbol_type,
            SymbolKind::Parameter { offset, register },
        )?;
        Ok(offset)
    }

    pub fn add_global_variable(&mut self, name: String, symbol_type: Type) -> Result<(), String> {
        // Add to global scope (first scope)
        if self.scopes[0].contains_key(&name) {
            return Err(format!("Global symbol '{name}' already defined"));
        }

        let symbol = Symbol {
            name: name.clone(),
            symbol_type,
            kind: SymbolKind::GlobalVariable,
        };

        self.scopes[0].insert(name, symbol);
        Ok(())
    }

    pub fn add_function(
        &mut self,
        name: String,
        return_type: Type,
        params: Vec<Parameter>,
    ) -> Result<(), String> {
        let function_type = Type::Function {
            ret_type: Box::new(return_type),
            params: params.clone(),
        };

        self.add_symbol(name, function_type, SymbolKind::Function { params })?;
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn get_local_offset(&self) -> i32 {
        self.local_offset
    }
}
