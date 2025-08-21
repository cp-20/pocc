use crate::{
    ir_generator::IRModule,
    lifetime_analyzer::{LifetimeTable, error::LifetimeAnalyzerError, function::analyze_function},
};

pub fn analyze_lifetime(module: &IRModule) -> Result<LifetimeTable, LifetimeAnalyzerError> {
    let functions = module
        .functions
        .iter()
        .map(analyze_function)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LifetimeTable { functions })
}
