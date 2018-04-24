//! Stores the neighbours of a node, managing what nodes to keep as neighbours.

use key::Key;
use node::Node;
use payload_handler::graph::key_space::{remove_duplicate_keys,
                                        sort_key_relative, KeySpace};

use slog::Logger;

/// Holds the neighbour store data
pub struct NeighboursStore {
    local_key_space: KeySpace,
    size: usize,
    neighbours: Vec<(Node, KeySpace)>,
    log: Logger,
}

impl NeighboursStore {
    /// Create a new neighbour store with a size and the key of the local node.
    pub fn new(
        local_key: Key,
        size: usize,
        key_space_size: usize,
        log: Logger,
    ) -> Self {
        let local_key_space = KeySpace::from_key(&local_key, key_space_size);
        info!(
            log,
            "Creating neighbours store";
            "local_key_space" => local_key_space.to_string());
        NeighboursStore {
            local_key_space: local_key_space,
            size: size,
            neighbours: vec![],
            log: log,
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
        info!(
            self.log,
            "Considering candidate neighbour";
            "node" => %node);

        let key_space =
            KeySpace::from_key(&node.key, self.local_key_space.get_size());

        // Don't add ourselves
        if key_space == self.local_key_space {
            return;
        }

        // Add the key to neighbours...
        self.neighbours.push((node.clone(), key_space));
        // ...sort the neighbours...
        sort_key_relative(
            &mut self.neighbours,
            &|&(_, ref ks)| ks.clone(),
            &self.local_key_space,
        );
        // ...remove duplicates...
        remove_duplicate_keys(&mut self.neighbours, &|&(_, ref ks)| ks.clone());
        // ...sort again because removing duplicates breaks ordering...
        // TODO: Fix
        sort_key_relative(
            &mut self.neighbours,
            &|&(_, ref ks)| ks.clone(),
            &self.local_key_space,
        );
        // ...remove the furthest neighbours.
        while self.neighbours.len() > self.size {
            self.neighbours.pop();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use key::Key;
    use address::Address;
    use slog;

    #[test]
    fn test_consider_candidates() {
        let test_log = Logger::root(slog::Discard, o!());

        let keys = vec![
            Key::new("00000001".to_string(), vec![1]),
            Key::new("00000002".to_string(), vec![2]),
            Key::new("00000003".to_string(), vec![3]),
            Key::new("00000004".to_string(), vec![4]),
            Key::new("00000005".to_string(), vec![5]),
            Key::new("00000006".to_string(), vec![6]),
            Key::new("00000007".to_string(), vec![7]),
        ];

        let mut ns =
            NeighboursStore::new(keys[keys.len() - 1].clone(), 3, 2, test_log);
        for i in 0..keys.len() - 1 {
            ns.consider_candidate(&Node::new(
                Address::new(vec![0, 0, 0, 0], 0),
                keys[i].clone(),
            ));
        }

        let mut data = ns.get_all()
            .iter()
            .map(|n| n.key.get_data()[0])
            .collect::<Vec<u8>>();
        data.sort();
        assert_eq!(data, vec![4, 5, 6]);
    }
}
