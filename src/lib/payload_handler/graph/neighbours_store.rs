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

        // New algorithm:
        // - Get distance from local to all neighbours
        // - Get min angle for each neighbour to other neighbours
        // - Normalise distances and angle
        // - Take top `n`, ranked by `angle - distance`

        // Don't add ourselves
        if key_space == self.local_key_space {
            return;
        }

        let neighbours_entry = (node.clone(), key_space);

        // Don't add a neighbour if it is already one
        if self.neighbours.contains(&neighbours_entry) {
            return;
        }

        // Add the key to neighbours
        self.neighbours.push(neighbours_entry);

        if self.neighbours.len() < self.max_num_neighbours {
            return;
        }

        let min_angles: Vec<f32> = self
            .neighbours
            .iter()
            .map(|&(_, ref ks)| {
                self.neighbours
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

        let distances: Vec<f32> = self
            .neighbours
            .iter()
            .map(|&(_, ref ks)| {
                self.key_space_manager.distance(&self.local_key_space, ks)
            })
            .collect();

        fn min_floats(v: &[f32]) -> f32 {
            *v.iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .expect("Error on unwrapping min")
        }
        fn max_floats(v: &[f32]) -> f32 {
            *v.iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .expect("Error on unwrapping max")
        }
        // let min_min_angle = min_floats(&min_angles);
        // let max_min_angle = max_floats(&min_angles);
        let min_distance = min_floats(&distances);
        let max_distance = max_floats(&distances);

        let distance_weighting = self.distance_weighting;
        let angle_weighting = self.angle_weighting;
        let scores = min_angles.iter().zip(&distances).map(|(a, d)| {
            assert!(&0.0 <= a && a <= &::std::f32::consts::PI);

            let normalized_a = 1.0 - (a / ::std::f32::consts::PI);

            let normalized_d =
                if (max_distance - min_distance).abs() > ::std::f32::EPSILON {
                    (d - min_distance) / (max_distance - min_distance)
                } else {
                    0.0
                };

            ((normalized_d * distance_weighting)
                + (normalized_a * angle_weighting))
                / (distance_weighting + angle_weighting)
        });

        let mut scores_map = HashMap::new();
        for (&(ref n, _), s) in self.neighbours.iter().zip(scores) {
            scores_map.insert(n.key.get_key_id().clone(), s);
        }

        self.neighbours.sort_by(|&(ref a, _), &(ref b, _)| {
            let a_score = scores_map[a.key.get_key_id()];
            let b_score = scores_map[b.key.get_key_id()];
            a_score.partial_cmp(&b_score).unwrap()
        });

        // ...remove the furthest neighbours.
        while self.neighbours.len() > self.max_num_neighbours {
            self.neighbours.pop();
        }
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
            .map(|n| n.key.get_data()[0])
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
            .map(|n| n.key.get_data()[3])
            .collect::<Vec<u8>>();
        data.sort();
        assert_that!(data).is_equal_to(vec![2, 6]);
    }
}
