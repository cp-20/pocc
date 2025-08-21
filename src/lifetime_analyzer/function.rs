use std::collections::{HashMap, HashSet};

use crate::{
    control_flow::ControlFlowGraph,
    ir_generator::IRFunction,
    lifetime_analyzer::{
        FunctionLifetime,
        domain::{IRAddress, LifetimeElement},
        element::analyze_elements,
        error::LifetimeAnalyzerError,
    },
    virtual_register::VirtualRegister,
};

#[derive(Debug, Clone, Default)]
pub struct RegisterCost {
    pub spill_cost: f32,
    pub function_call_penalty: f32,
    pub loop_penalty: f32,
    pub total_cost: f32,
}

impl RegisterCost {
    fn add_usage(&mut self, weight: f32) {
        self.spill_cost += weight;
        self.update_total();
    }

    fn add_function_call_penalty(&mut self, penalty: f32) {
        self.function_call_penalty += penalty;
        self.update_total();
    }

    fn add_loop_penalty(&mut self, penalty: f32) {
        self.loop_penalty += penalty;
        self.update_total();
    }

    fn update_total(&mut self) {
        self.total_cost = self.spill_cost + self.function_call_penalty + self.loop_penalty;
    }
}

pub type RegisterCosts = HashMap<VirtualRegister, RegisterCost>;

fn collect_instructions_by_block(elements: &[LifetimeElement]) -> HashMap<usize, Vec<IRAddress>> {
    let mut instructions_by_block: HashMap<usize, Vec<IRAddress>> = HashMap::new();
    for elem in elements {
        for addr in elem.assigns.iter().chain(elem.references.iter()) {
            instructions_by_block
                .entry(addr.id)
                .or_default()
                .push(addr.clone());
        }
    }
    for instructions in instructions_by_block.values_mut() {
        instructions.sort_by_key(|addr| addr.offset);
        instructions.dedup_by_key(|addr| addr.offset);
    }
    instructions_by_block
}

fn build_successor_map(
    instructions_by_block: &HashMap<usize, Vec<IRAddress>>,
    flow: &ControlFlowGraph,
) -> HashMap<IRAddress, Vec<IRAddress>> {
    let mut successors_map: HashMap<IRAddress, Vec<IRAddress>> = HashMap::new();
    let mut first_instructions: HashMap<usize, IRAddress> = HashMap::new();

    for (&block, vec) in instructions_by_block {
        if let Some(first) = vec.first() {
            first_instructions.insert(block, first.clone());
        }
        for window in vec.windows(2) {
            let curr = &window[0];
            let next = &window[1];
            successors_map
                .entry(curr.clone())
                .or_default()
                .push(next.clone());
        }
    }

    // Last-in-block successors
    for (&block_id, vec) in instructions_by_block {
        let Some(last) = vec.last() else {
            continue;
        };

        let successors = flow.successors(block_id);
        for successor_block_id in successors {
            if let Some(first) = first_instructions.get(&successor_block_id) {
                successors_map
                    .entry(last.clone())
                    .or_default()
                    .push(first.clone());
            }
        }
    }

    successors_map
}

fn build_def_use_maps(
    elements: &[LifetimeElement],
) -> (
    HashMap<IRAddress, HashSet<VirtualRegister>>,
    HashMap<IRAddress, HashSet<VirtualRegister>>,
) {
    let mut defs: HashMap<IRAddress, HashSet<VirtualRegister>> = HashMap::new();
    let mut uses: HashMap<IRAddress, HashSet<VirtualRegister>> = HashMap::new();

    for elem in elements {
        for addr in &elem.assigns {
            defs.entry(addr.clone())
                .or_default()
                .insert(elem.reg.clone());
        }
        for addr in &elem.references {
            uses.entry(addr.clone())
                .or_default()
                .insert(elem.reg.clone());
        }
    }

    (defs, uses)
}

fn compute_liveness(
    instructions_by_block: &HashMap<usize, Vec<IRAddress>>,
    successors_map: &HashMap<IRAddress, Vec<IRAddress>>,
    defs: &HashMap<IRAddress, HashSet<VirtualRegister>>,
    uses: &HashMap<IRAddress, HashSet<VirtualRegister>>,
) -> (
    HashMap<IRAddress, HashSet<VirtualRegister>>,
    HashMap<IRAddress, HashSet<VirtualRegister>>,
) {
    let mut live_in: HashMap<IRAddress, HashSet<VirtualRegister>> = HashMap::new();
    let mut live_out: HashMap<IRAddress, HashSet<VirtualRegister>> = HashMap::new();

    // initialize empty sets for all nodes
    for addrs in instructions_by_block.values() {
        for addr in addrs {
            live_in.entry(addr.clone()).or_default();
            live_out.entry(addr.clone()).or_default();
        }
    }

    // Iterative backward dataflow
    let mut changed = true;
    while changed {
        changed = false;
        let addrs_vec: Vec<_> = instructions_by_block.values().collect();
        for addrs in addrs_vec.iter().rev() {
            for addr in addrs.iter().rev() {
                let out_set: HashSet<VirtualRegister> = successors_map
                    .get(addr)
                    .map(|successors| {
                        successors
                            .iter()
                            .flat_map(|s| live_in.get(s).unwrap().iter().cloned())
                            .collect()
                    })
                    .unwrap_or_default();

                let out_set_prev = live_out.get(addr).unwrap().clone();

                // compute IN and OUT
                let defs_here = defs.get(addr).cloned().unwrap_or_default();
                let uses_here = uses.get(addr).cloned().unwrap_or_default();
                let in_new: HashSet<_> = uses_here
                    .union(&out_set.difference(&defs_here).cloned().collect())
                    .cloned()
                    .collect();
                if &in_new != live_in.get(addr).unwrap() {
                    live_in.insert(addr.clone(), in_new);
                    changed = true;
                }
                if out_set != out_set_prev {
                    live_out.insert(addr.clone(), out_set);
                    changed = true;
                }
            }
        }
    }

    (live_in, live_out)
}

fn detect_loop_blocks(flow: &ControlFlowGraph) -> HashSet<usize> {
    let mut loop_blocks = HashSet::new();

    // Simple loop detection: blocks that have back edges (successors with smaller or equal IDs)
    for block_id in flow.blocks.clone() {
        let successors = flow.successors(block_id);
        for &successor in &successors {
            if successor <= block_id {
                // Potential back edge - mark both blocks as loop blocks
                loop_blocks.insert(block_id);
                loop_blocks.insert(successor);

                // Mark all blocks between successor and block_id as loop blocks
                for i in successor..=block_id {
                    loop_blocks.insert(i);
                }
            }
        }
    }

    loop_blocks
}

fn has_function_call(function: &IRFunction, addr: &IRAddress) -> bool {
    if let Some(block) = function.body.get(addr.id) {
        if let Some(instruction) = block.nodes.get(addr.offset) {
            // Check if instruction is a function call
            // This depends on your IR structure - adapt as needed
            return instruction.to_string().contains("call")
                || instruction.to_string().contains("invoke");
        }
    }
    false
}

fn calculate_register_costs(
    function: &IRFunction,
    elements: &[LifetimeElement],
    live_in: &HashMap<IRAddress, HashSet<VirtualRegister>>,
    live_out: &HashMap<IRAddress, HashSet<VirtualRegister>>,
    flow: &ControlFlowGraph,
) -> RegisterCosts {
    let mut costs: RegisterCosts = HashMap::new();
    let loop_blocks = detect_loop_blocks(flow);

    // Initialize costs for all registers
    for elem in elements {
        costs.entry(elem.reg.clone()).or_default();
    }

    // Calculate costs based on liveness
    for (addr, live_regs) in live_in {
        let is_in_loop = loop_blocks.contains(&addr.id);
        let has_call = has_function_call(function, addr);

        for reg in live_regs {
            let cost = costs.entry(reg.clone()).or_default();

            // Base usage cost
            let base_weight = if is_in_loop { 10.0 } else { 1.0 };
            cost.add_usage(base_weight);

            // Loop penalty (higher cost for spilling in loops)
            if is_in_loop {
                cost.add_loop_penalty(50.0);
            }

            // Function call penalty (caller-save registers need saving)
            if has_call {
                cost.add_function_call_penalty(20.0);
            }
        }
    }

    // Add costs for live-out registers at function calls
    for (addr, live_regs) in live_out {
        if has_function_call(function, addr) {
            for reg in live_regs {
                let cost = costs.entry(reg.clone()).or_default();
                cost.add_function_call_penalty(15.0);
            }
        }
    }

    costs
}

pub fn analyze_function(function: &IRFunction) -> Result<FunctionLifetime, LifetimeAnalyzerError> {
    let elements = analyze_elements(function)?;
    let flow = ControlFlowGraph::from_function(function);

    let instructions_by_block = collect_instructions_by_block(&elements);

    let successors_map = build_successor_map(&instructions_by_block, &flow);

    let (defs, uses) = build_def_use_maps(&elements);

    let (live_in, live_out) =
        compute_liveness(&instructions_by_block, &successors_map, &defs, &uses);

    // TODO: Implement the actual cost calculation logic
    let _costs = calculate_register_costs(function, &elements, &live_in, &live_out, &flow);

    Ok(FunctionLifetime {
        name: function.name.clone(),
        elements: elements.clone(),
        live_in,
        live_out,
    })
}
