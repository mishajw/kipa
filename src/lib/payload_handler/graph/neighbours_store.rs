//! Stores the neighbours of a node, managing what nodes to keep as neighbours

use key::Key;
use node::Node;
use payload_handler::graph::key_space::{KeySpace, KeySpaceManager};

use slog::Logger;
use std::collections::HashMap;
use std::sync::Arc;

/// The default size of the neighbours store
pub const DEFAULT_MAX_NUM_NEIGHBOURS: &str = "3";

/// The default weight for distance when considering neighbours
pub const DEFAULT_DISTANCE_WEIGHTING: &str = "0.5";

/// The default weight for angle when considering neighbours
pub const DEFAULT_ANGLE_WEIGHTING: &str = "0.5";

/// Holds the neighbour store data
pub struct NeighboursStore {
    local_key_space: KeySpace,
    key_space_manager: Arc<KeySpaceManager>,
    max_num_neighbours: usize,
    distance_weighting: f32,
    angle_weighting: f32,
    neighbours: Vec<(Node, KeySpace)>,
    log: Logger,
}

impl NeighboursStore {
    /// Create a new neighbour store with a maximum number of neighbours and the
    /// key of the local node
    pub fn new(
        local_key: &Key,
        max_num_neighbours: usize,
        distance_weighting: f32,
        angle_weighting: f32,
        key_space_manager: Arc<KeySpaceManager>,
        log: Logger,
    ) -> Self
    {
        let local_key_space = key_space_manager.create_from_key(local_key);
        info!(
            log,
            "Creating neighbours store";
            "local_key_space" => local_key_space.to_string());
        NeighboursStore {
            local_key_space,
            key_space_manager,
            max_num_neighbours,
            distance_weighting,
            angle_weighting,
            neighbours: Vec::new(),
            log,
        }
    }

    /// Get the `n` closest neighbours to some key
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

    /// Get all neightbours
    pub fn get_all(&self) -> Vec<Node> {
        self.neighbours
            .iter()
            .map(|&(ref n, _)| n.clone())
            .collect()
    }

    /// Given a node, consider keeping it as a neighbour
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

        let neighbours_entry = (node.clone(), key_space);

        // Check if there is an existing neighbour with the same key - if there
        // is, check if the address needs updating. If we find any matching
        // nodes, exit the function
        //
        // TODO: This opens up a vulnerability where a malicious node can reply
        // to query requests with real keys but fake addresses, overriding
        // the daemon's neighbour entry
        let mut found_duplicate_node = false;
        self.neighbours.iter_mut().for_each(|(ref mut n, ref _ks)| {
            if n.key == node.key {
                found_duplicate_node = true;
                if n.address != node.address {
                    n.address = node.address.clone();
                }
            }
        });
        if found_duplicate_node {
            return;
        }

        // If we have space for the new node, add it and return
        if self.neighbours.len() < self.max_num_neighbours {
            self.neighbours.push(neighbours_entry);
            return;
        }

        let mut potential_neighbours = self.neighbours.clone();
        potential_neighbours.push(neighbours_entry.clone());
        let scores = self.get_neighbour_scores(&potential_neighbours);
        let (min_key_id, _) = scores
            .iter()
            .min_by(|(_, a_score), (_, b_score)| {
                b_score.partial_cmp(a_score).unwrap()
            })
            // We can be certain of a result, as `potential_neighbours` has at
            // least one element in it
            .unwrap();

        // If the new node has *not* got the worst score, remove the node with
        // the worst score and add the new node
        if min_key_id != &node.key.key_id {
            self.neighbours
                .retain(|(node, _)| &node.key.key_id != min_key_id);
            self.neighbours.push(neighbours_entry);
        }

        debug_assert!(self.neighbours.len() <= self.max_num_neighbours);
    }

    /// Remove a neighbour by its key
    pub fn remove_by_key(&mut self, key: &Key) {
        self.neighbours.retain(|(n, _)| &n.key != key);
    }

    /// Get the scores of each neighbour
    ///
    /// This is calculated as a weighted score of:
    /// - The distance in keyspace between the local node and the neighbour node
    /// - How "unique" the angle between the local node and the neighbour node
    ///   is, i.e. does adding this neighbour add a link in a new direction?
    fn get_neighbour_scores(
        &mut self,
        neighbours: &Vec<(Node, KeySpace)>,
    ) -> HashMap<String, f32>
    {
        // Calculate the angle metric
        let min_angles: Vec<f32> = neighbours
            .iter()
            .map(|&(_, ref ks)| {
                neighbours
                    .iter()
                    .filter(|&&(_, ref ks2)| ks != ks2)
                    .map(|&(_, ref ks2)| {
                        self.key_space_manager.angle(
                            &self.local_key_space,
                            &ks,
                            &ks2,
                        )
                    })
                    .min_by(|a, b| {
                        a.partial_cmp(b).expect("Error on comparing angles")
                    })
                    .unwrap()
            })
            .collect();

        // Calculate the distance metric
        let distances: Vec<f32> = neighbours
            .iter()
            .map(|&(_, ref ks)| {
                self.key_space_manager.distance(&self.local_key_space, ks)
            })
            .collect();

        // Calculate the min/max distances for scaling
        let min_distance = distances
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .expect("Error on unwrapping min distance");
        let max_distance = distances
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .expect("Error on unwrapping max distance");

        // Calculate the scores of each neighbour
        let scores = min_angles.iter().zip(&distances).map(|(a, d)| {
            assert!(&0.0 <= a && a <= &::std::f32::consts::PI);

            let normalized_a = 1.0 - (a / ::std::f32::consts::PI);
            let normalized_d =
                if (max_distance - min_distance).abs() > ::std::f32::EPSILON {
                    (d - min_distance) / (max_distance - min_distance)
                } else {
                    0.0
                };

            ((normalized_d * self.distance_weighting)
                + (normalized_a * self.angle_weighting))
                / (self.distance_weighting + self.angle_weighting)
        });

        // Put the scores into a map
        let mut scores_map = HashMap::new();
        for (&(ref n, _), s) in neighbours.iter().zip(scores) {
            scores_map.insert(n.key.key_id.clone(), s);
        }
        scores_map
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use address::Address;
    use key::Key;

    use slog;
    use spectral::assert_that;

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
            &keys[keys.len() - 1],
            3,
            1.0,
            0.0,
            Arc::new(KeySpaceManager::new(1)),
            test_log,
        );
        for i in 0..keys.len() - 1 {
            ns.consider_candidate(&Node::new(
                Address::new(vec![0, 0, 0, 0], 0),
                keys[i].clone(),
            ));
        }

        let mut data = ns
            .get_all()
            .iter()
            .map(|n| n.key.data[0])
            .collect::<Vec<u8>>();
        data.sort();
        assert_that!(data).is_equal_to(vec![4, 5, 6]);
    }

    #[test]
    fn test_consider_candidates_angles() {
        let test_log = Logger::root(slog::Discard, o!());

        let keys = vec![
            Key::new("00000001".to_string(), vec![0, 0, 0, 1]),
            Key::new("00000002".to_string(), vec![0, 0, 0, 2]),
            Key::new("00000003".to_string(), vec![0, 0, 0, 3]),
            Key::new("00000004".to_string(), vec![0, 0, 0, 6]),
        ];

        let mut ns = NeighboursStore::new(
            &keys[2],
            2,
            0.5,
            0.5,
            Arc::new(KeySpaceManager::new(1)),
            test_log,
        );

        for k in keys {
            ns.consider_candidate(&Node::new(
                Address::new(vec![0, 0, 0, 0], 0),
                k.clone(),
            ));
        }

        let mut data = ns
            .get_all()
            .iter()
            .map(|n| n.key.data[3])
            .collect::<Vec<u8>>();
        data.sort();
        assert_that!(data).is_equal_to(vec![2, 6]);
    }
}
