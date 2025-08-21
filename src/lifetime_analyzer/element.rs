use crate::{
    ir_generator::{IRFunction, IRNodeKind, IRValue, IRVariable},
    lifetime_analyzer::{IRAddress, LifetimeElement, error::LifetimeAnalyzerError},
};

pub fn analyze_elements(
    function: &IRFunction,
) -> Result<Vec<LifetimeElement>, LifetimeAnalyzerError> {
    let mut elements: Vec<LifetimeElement> = Vec::new();

    for param_reg in &function.parameter_regs {
        elements.push(LifetimeElement {
            reg: param_reg.clone(),
            references: vec![IRAddress::new(function.body[0].id, 1)],
            assigns: vec![IRAddress::new(function.body[0].id, 0)],
        });
    }

    for param in &function.parameters {
        elements.push(LifetimeElement {
            reg: param.clone(),
            references: vec![],
            assigns: vec![IRAddress::new(function.body[0].id, 1)],
        });
    }

    for block in &function.body {
        for (offset, node) in block.nodes.iter().enumerate() {
            let current_address = IRAddress::new(block.id, offset);
            match &node.kind {
                IRNodeKind::Assign { variable, value } => {
                    analyze_assign(&mut elements, variable, &current_address)?;
                    analyze_ref(&mut elements, value, &current_address)?;
                }
                IRNodeKind::AddressAssignment { address, value } => {
                    analyze_ref(&mut elements, address, &current_address)?;
                    analyze_ref(&mut elements, value, &current_address)?;
                }
                IRNodeKind::Lea {
                    base,
                    index,
                    result,
                    ..
                } => {
                    analyze_ref(&mut elements, base, &current_address)?;
                    analyze_ref(
                        &mut elements,
                        &IRValue::Register(index.clone()),
                        &current_address,
                    )?;
                    analyze_assign(
                        &mut elements,
                        &IRVariable::Register(result.clone()),
                        &current_address,
                    )?;
                }
                IRNodeKind::BinaryOp {
                    left,
                    right,
                    result,
                    ..
                } => {
                    analyze_ref(&mut elements, left, &current_address)?;
                    analyze_ref(&mut elements, right, &current_address)?;
                    analyze_assign(
                        &mut elements,
                        &IRVariable::Register(result.clone()),
                        &current_address,
                    )?;
                }
                IRNodeKind::UnaryOp {
                    operand, result, ..
                } => {
                    analyze_ref(&mut elements, operand, &current_address)?;
                    analyze_assign(
                        &mut elements,
                        &IRVariable::Register(result.clone()),
                        &current_address,
                    )?;
                }
                IRNodeKind::FunctionCall {
                    name,
                    arguments,
                    result,
                    argument_regs,
                } => {
                    for arg in arguments {
                        analyze_ref(&mut elements, arg, &current_address.prev())?;
                    }
                    for arg_reg in argument_regs {
                        analyze_assign(
                            &mut elements,
                            &IRVariable::Register(arg_reg.clone()),
                            &current_address.prev(),
                        )?;
                        analyze_ref(
                            &mut elements,
                            &IRValue::Register(arg_reg.clone()),
                            &current_address,
                        )?;
                    }
                    if let Some(register) = name.get_register() {
                        analyze_ref(
                            &mut elements,
                            &IRValue::Register(register.clone()),
                            &current_address,
                        )?;
                    }
                    if let Some(result_reg) = result {
                        analyze_assign(
                            &mut elements,
                            &IRVariable::Register(result_reg.clone()),
                            &current_address,
                        )?;
                    }
                }
                IRNodeKind::Branch {
                    condition: Some(condition),
                    ..
                } => {
                    analyze_ref(&mut elements, condition, &current_address)?;
                }
                IRNodeKind::Return { value: Some(value) } => {
                    analyze_ref(&mut elements, value, &current_address)?;
                }
                _ => {}
            }
        }
    }

    Ok(elements)
}

fn analyze_assign(
    elements: &mut Vec<LifetimeElement>,
    variable: &IRVariable,
    address: &IRAddress,
) -> Result<(), LifetimeAnalyzerError> {
    let Some(register) = variable.get_register() else {
        return Ok(());
    };

    if let Some(element) = elements.iter_mut().find(|e| e.reg == register) {
        element.assigns.push(address.clone());
    } else {
        elements.push(LifetimeElement {
            reg: register,
            references: vec![],
            assigns: vec![address.clone()],
        });
    }

    Ok(())
}

fn analyze_ref(
    elements: &mut Vec<LifetimeElement>,
    value: &IRValue,
    address: &IRAddress,
) -> Result<(), LifetimeAnalyzerError> {
    let Some(register) = value.get_register() else {
        return Ok(());
    };

    // string literals are initialized when they are first referenced
    if matches!(value, IRValue::StringLiteral { .. }) && !elements.iter().any(|e| e.reg == register)
    {
        elements.push(LifetimeElement {
            reg: register.clone(),
            references: vec![address.clone()],
            assigns: vec![address.prev().clone()],
        });
    }

    let Some(element) = elements.iter_mut().find(|e| e.reg == register) else {
        return Err(LifetimeAnalyzerError::new(format!(
            "Register {register} not found in lifetime elements"
        )));
    };

    element.references.push(address.clone());

    Ok(())
}
