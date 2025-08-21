use crate::{
    ir_generator::IRModule,
    ir_optimizer::{
        assign::remove_redundant_assignments, error::IROptimizerError, inline::inline_functions,
        propagator::propagate_constants,
    },
};

pub fn optimize_ir(module: &IRModule) -> Result<IRModule, IROptimizerError> {
    let mut optimized_module = module.clone();

    remove_redundant_assignments(&mut optimized_module)?;
    propagate_constants(&mut optimized_module)?;

    if inline_functions(&mut optimized_module)? {
        remove_redundant_assignments(&mut optimized_module)?;
        propagate_constants(&mut optimized_module)?;
    };

    Ok(optimized_module)
}
