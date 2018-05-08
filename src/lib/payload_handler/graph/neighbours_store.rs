//! Stores the neighbours of a node, managing what nodes to keep as neighbours.

use key::Key;
use node::Node;
use payload_handler::graph::key_space::{KeySpace, KeySpaceManager};

use slog::Logger;
use std::sync::Arc;

/// Holds the neighbour store data
pub struct NeighboursStore {
    local_key_space: KeySpace,
    key_space_manager: Arc<KeySpaceManager>,
    size: usize,
    neighbours: Vec<(Node, KeySpace)>,
    log: Logger,
}

impl NeighboursStore {
    /// Create a new neighbour store with a size and the key of the local node.
    pub fn new(
        local_key: Key,
        size: usize,
        key_space_manager: Arc<KeySpaceManager>,
        log: Logger,
    ) -> Self {
        let local_key_space = key_space_manager.create_from_key(&local_key);
        info!(
            log,
            "Creating neighbours store";
            "local_key_space" => local_key_space.to_string());
        NeighboursStore {
            local_key_space: local_key_space,
            key_space_manager: key_space_manager,
            size: size,
            neighbours: vec![],
            log: log,
        }
    }

    /// Get the `n` closest neighbours to some key.
    pub fn get_n_closest(&self, key: &Key, n: usize) -> Vec<Node> {
        let mut neighbours = self.neighbours.clone();
        self.key_space_manager.sort_key_relative(
            &mut neighbours,
            &|&(_, ref ks)| ks.clone(),
            &self.key_space_manager.create_from_key(key),
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
        let key_space = self.key_space_manager.create_from_key(&node.key);

        info!(
            self.log,
            "Considering candidate neighbour";
            "node" => %node,
            "distance" => self.key_space_manager.distance(
                &key_space, &self.local_key_space));

        // Don't add ourselves
        if key_space == self.local_key_space {
            return;
        }

        // Add the key to neighbours...
        self.neighbours.push((node.clone(), key_space));
        // ...sort the neighbours...
        self.key_space_manager.sort_key_relative(
            &mut self.neighbours,
            &|&(_, ref ks)| ks.clone(),
            &self.local_key_space,
        );
        // ...remove duplicates...
        self.key_space_manager
            .remove_duplicate_keys(&mut self.neighbours, &|&(_, ref ks)| {
                ks.clone()
            });
        // ...sort again because removing duplicates breaks ordering...
        // TODO: Fix
        self.key_space_manager.sort_key_relative(
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

        let mut ns = NeighboursStore::new(
            keys[keys.len() - 1].clone(),
            3,
            Arc::new(KeySpaceManager::new(1)),
            test_log,
        );
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
