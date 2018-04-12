//! Stores the neighbours of a node, managing what nodes to keep as neighbours.

use key::Key;
use node::Node;
use request_handler::graph::key_space::KeySpace;

/// Holds the neighbour store data
pub struct NeighboursStore {
    local_key_space: KeySpace,
    size: usize,
    neighbours: Vec<(Node, KeySpace)>,
}

impl NeighboursStore {
    /// Create a new neighbour store with a size and the key of the local node.
    pub fn new(local_key: Key, size: usize) -> Self {
        let local_key_space = KeySpace::from_key(&local_key, 2);
        NeighboursStore {
            local_key_space: local_key_space,
            size: size,
            neighbours: vec![],
        }
    }

    /// Get the `n` closest neighbours to some key.
    pub fn get_n_closest(&self, key: &Key, n: usize) -> Vec<Node> {
        let mut neighbours = self.neighbours.clone();
        Self::sort_key_relative(&mut neighbours, KeySpace::from_key(key, 2));
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
        self.neighbours
            .push((node.clone(), KeySpace::from_key(&node.key, 2)));
        Self::sort_key_relative(
            &mut self.neighbours,
            self.local_key_space.clone(),
        );

        while self.neighbours.len() > self.size {
            self.neighbours.pop();
        }
    }

    fn sort_key_relative(v: &mut Vec<(Node, KeySpace)>, key_space: KeySpace) {
        v.sort_by(|&(_, ref a_ks), &(_, ref b_ks)| {
            (a_ks - &key_space)
                .partial_cmp(&(b_ks - &key_space))
                .unwrap()
        })
    }
}
