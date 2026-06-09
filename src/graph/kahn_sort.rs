use crate::ast::ebnf_06::NodeDecl;
use std::collections::{HashMap, HashSet, VecDeque};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Kahn's algorithm — layered topological sort
// Returns Ok(layers) or Err(nodes_in_cycle)
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub fn kahn_sort(
    nodes: &HashMap<String, NodeDecl>,
    edges: &[(String, String)],
) -> Result<Vec<Vec<String>>, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> =
        nodes.keys().map(|n| (n.clone(), Vec::new())).collect();
    let mut in_deg: HashMap<String, usize> = nodes.keys().map(|n| (n.clone(), 0)).collect();

    for (src, dst) in edges {
        adj.entry(src.clone()).or_default().push(dst.clone());
        *in_deg.entry(dst.clone()).or_insert(0) += 1;
    }

    let mut queue: VecDeque<String> = in_deg
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();

    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    while !queue.is_empty() {
        let layer_size = queue.len();
        let mut layer = Vec::with_capacity(layer_size);

        for _ in 0..layer_size {
            let node = queue.pop_front().unwrap();
            for succ in adj.get(&node).into_iter().flatten() {
                let deg = in_deg.get_mut(succ).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(succ.clone());
                }
            }
            visited.insert(node.clone());
            layer.push(node);
        }

        layer.sort();
        layers.push(layer);
    }

    if visited.len() < nodes.len() {
        let mut cycle: Vec<String> = nodes
            .keys()
            .filter(|n| !visited.contains(*n))
            .cloned()
            .collect();
        cycle.sort();
        return Err(cycle);
    }

    Ok(layers)
}
