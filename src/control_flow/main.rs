use crate::ir_generator::{IRFunction, IRNodeKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ControlFlowGraph {
    pub blocks: Vec<usize>,
    pub edges: Vec<(usize, usize)>,
}

impl ControlFlowGraph {
    fn new() -> Self {
        ControlFlowGraph {
            blocks: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn add_block(&mut self, block_id: usize) {
        if !self.blocks.contains(&block_id) {
            self.blocks.push(block_id);
        }
    }

    fn add_edge(&mut self, from: usize, to: usize) {
        if !self.edges.contains(&(from, to)) {
            self.edges.push((from, to));
        }
    }

    pub fn successors(&self, block_id: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|(from, _)| *from == block_id)
            .map(|(_, to)| *to)
            .collect()
    }
}

impl ControlFlowGraph {
    pub fn from_function(function: &IRFunction) -> Self {
        let mut flow = ControlFlowGraph::new();
        for block in &function.body {
            if let Some(last_id) = flow.blocks.last() {
                let last_block = function.body.iter().find(|b| b.id == *last_id).unwrap();
                if last_block.is_connected_to_next() {
                    flow.add_edge(*last_id, block.id);
                }
            }
            flow.add_block(block.id);
            for node in &block.nodes {
                if let IRNodeKind::Branch {
                    true_branch,
                    false_branch,
                    ..
                } = &node.kind
                {
                    if let Some(true_branch) = true_branch {
                        flow.add_edge(block.id, *true_branch);
                    }
                    if let Some(false_branch) = false_branch {
                        flow.add_edge(block.id, *false_branch);
                    }
                }
            }
        }
        flow
    }
}
