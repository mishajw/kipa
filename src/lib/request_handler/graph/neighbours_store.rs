//! Stores the neighbours of a node, managing what nodes to keep as neighbours.

use key::Key;
use node::Node;
use request_handler::graph::key_space::{sort_key_relative, KeySpace};

/// Holds the neighbour store data
pub struct NeighboursStore {
    local_key_space: KeySpace,
    size: usize,
    neighbours: Vec<(Node, KeySpace)>,
}

impl NeighboursStore {
    /// Create a new neighbour store with a size and the key of the local node.
    pub fn new(local_key: Key, size: usize, key_space_size: usize) -> Self {
        let local_key_space = KeySpace::from_key(&local_key, key_space_size);
        NeighboursStore {
            local_key_space: local_key_space,
            size: size,
            neighbours: vec![],
        }
    }

    /// Get the `n` closest neighbours to some key.
    pub fn get_n_closest(&self, key: &Key, n: usize) -> Vec<Node> {
        let mut neighbours = self.neighbours.clone();
        sort_key_relative(
            &mut neighbours,
            &|&(_, ref ks)| ks.clone(),
            &KeySpace::from_key(key, self.local_key_space.get_size()),
        );
        neighbours
            .iter()
            .take(n)
            .map(|&(ref n, _)| n.clone())
            .collect()
    }

    /// Get all neightbours.
    pub fn get_all(&self) -> Vec<Node> {
        self.neighbours
            .iter()
            .map(|&(ref n, _)| n.clone())
            .collect()
    }

    /// Given a node, consider keeping it as a neighbour.
    pub fn consider_candidate(&mut self, node: &Node) {
        trace!("Considering candidate neighbour: {}", node);

        self.neighbours.push((
            node.clone(),
            KeySpace::from_key(&node.key, self.local_key_space.get_size()),
        ));
        sort_key_relative(
            &mut self.neighbours,
            &|&(_, ref ks)| ks.clone(),
            &self.local_key_space,
        );

        while self.neighbours.len() > self.size {
            self.neighbours.pop();
        }
    }
}
