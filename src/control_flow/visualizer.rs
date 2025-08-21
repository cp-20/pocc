use std::collections::HashMap;

use crate::control_flow::main::ControlFlowGraph;

impl std::fmt::Display for ControlFlowGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut edge_map: HashMap<_, Vec<_>> = HashMap::new();
        for (from, to) in &self.edges {
            edge_map.entry(from).or_default().push(to);
        }

        let mut froms: Vec<_> = edge_map.into_iter().collect();
        froms.sort_by_key(|(from, _)| *from);

        for (from, tos) in froms {
            let tos_str = tos
                .iter()
                .map(|to| format!("p{to}"))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(f, "p{from} -> {tos_str}")?;
        }
        Ok(())
    }
}
