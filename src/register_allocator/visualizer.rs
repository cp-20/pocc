use crate::register_allocator::RegisterAllocationFunction;

impl std::fmt::Display for RegisterAllocationFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut entries: Vec<_> = self.mapping.iter().collect();
        entries.sort_by_key(|(var, _)| *var);
        for (var, reg) in entries {
            writeln!(f, "{} -> {}", var, reg.name())?;
        }
        let mut used_callee_save: Vec<_> = self.used_callee_save.iter().collect();
        used_callee_save.sort();
        used_callee_save
            .into_iter()
            .try_for_each(|reg| writeln!(f, "Callee-save: {}", reg.name()))?;
        writeln!(f, "Max spilled registers: {}", self.max_spilled_registers)?;
        Ok(())
    }
}
