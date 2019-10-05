//! Key space management structure.

use api::{Key, KeySpace};

use std::io::Cursor;
use std::mem::size_of;
use std::ops::{BitXor, Deref};

use byteorder::{BigEndian, ReadBytesExt};

/// The default number of dimensions in key space.
pub const DEFAULT_KEY_SPACE_SIZE: &str = "2";

/// Manages locations in `KeySpace`, including their creation and distance metrics.
pub struct KeySpaceManager {
    num_key_space_dims: usize,
}

impl KeySpaceManager {
    #[allow(missing_docs)]
    pub fn new(num_key_space_dims: usize) -> Self {
        KeySpaceManager { num_key_space_dims }
    }

    /// Creates a location in key space from a key.
    pub fn create_from_key(&self, key: &Key) -> KeySpace {
        let chunk_size = (size_of::<i32>() * self.num_key_space_dims) / size_of::<u8>();
        let chunks = key.data.chunks(chunk_size);
        let mut chunks_transpose = vec![vec![]; chunk_size];
        for cs in chunks {
            for (i, c) in cs.iter().enumerate() {
                chunks_transpose[i].push(c);
            }
        }
        let coords: Vec<i32> = chunks_transpose
            .iter()
            .map(|cs| cs.iter().map(Deref::deref).fold(0 as u8, BitXor::bitxor))
            .collect::<Vec<u8>>()
            .chunks(size_of::<i32>() / size_of::<u8>())
            .map(|cs| Cursor::new(cs).read_i32::<BigEndian>().unwrap())
            .collect();
        KeySpace { coords }
    }

    /// Calculates the euclidean distance between points in key space.
    pub fn distance(&self, a_ks: &KeySpace, b_ks: &KeySpace) -> f32 {
        use std::cmp::min;
        static I32_RANGE: i64 = (::std::i32::MAX as i64) - (::std::i32::MIN as i64);

        assert_eq!(a_ks.coords.len(), b_ks.coords.len());

        let total: i64 = a_ks
            .coords
            .iter()
            .zip(&b_ks.coords)
            // Map to `i64` so we have enough space to subtract `i32`s
            .map(|(a, b)| (i64::from(*a), i64::from(*b)))
            .map(|(a, b)| {
                let diff = (a - b).abs();
                min(diff, I32_RANGE - diff)
            })
            .sum();

        let result = (total as f32).powf(1f32 / a_ks.coords.len() as f32);
        assert!(result >= 0.0);
        result
    }

    /// Calculates the angle between two points in key space `a` and `b`, relative to a point in key
    /// space `relative_to`.
    // TODO: Is this function unused?
    pub fn angle(&self, relative_to: &KeySpace, a: &KeySpace, b: &KeySpace) -> f32 {
        let dot = |a2: &KeySpace, b2: &KeySpace| -> f32 {
            let result: i128 = a2
                .coords
                .iter()
                .zip(&b2.coords)
                .zip(&relative_to.coords)
                // Map to `i128` so we have enough space to subtract `i32`s,
                // multiply together, and sum results
                .map(|((i, j), l)| ((i128::from(*i), i128::from(*j)), i128::from(*l)))
                .map(|((i, j), l)| (i - l, j - l))
                .map(|(i, j)| i * j)
                .sum();
            result as f32
        };

        let numerator = dot(a, b);
        let denominator = dot(a, a).sqrt() * dot(b, b).sqrt();

        // If the denominator is zero, then either `a` or `b` are equal to
        // `relative_to`, and the angle between `a` and `b` is zero too.
        if denominator == 0.0 {
            return 0.0;
        }

        let cos_angle = numerator / denominator;

        // Ensure that the angle is between -1 and 1.
        // Check just around this range to allow for some floating point error.
        assert!(cos_angle.abs() < 1.01);

        cos_angle.min(1.0).max(-1.0).acos()
    }

    /// Sort a vector by each element's distance to some key in key space
    pub fn sort_key_relative<T>(
        &self,
        v: &mut Vec<T>,
        get_key_space_fn: &Fn(&T) -> KeySpace,
        key_space: &KeySpace,
    ) {
        // TODO: Can we use lifetimes to avoid `get_key_space_fn` returning a value, and instead a
        // reference? Related: https://github.com/rust-lang/rust/issues/22340
        v.sort_by(|a: &T, b: &T| {
            let a_ks: KeySpace = get_key_space_fn(a);
            let b_ks: KeySpace = get_key_space_fn(b);
            (self.distance(&a_ks, key_space))
                .partial_cmp(&(self.distance(&b_ks, key_space)))
                .unwrap()
        });
    }

    /// Remove elements from a vector that contain the duplicate keys.
    pub fn remove_duplicate_keys<T>(&self, v: &mut Vec<T>, get_key_space_fn: &Fn(&T) -> KeySpace) {
        if v.len() <= 1 {
            return;
        }

        let sort_key = &get_key_space_fn(&v[0]);
        self.sort_key_relative(v, &get_key_space_fn, sort_key);

        for i in (1..v.len()).rev() {
            let k1 = get_key_space_fn(&v[i]);
            let k2 = get_key_space_fn(&v[i - 1]);
            if k1 == k2 {
                v.swap_remove(i);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spectral::assert_that;
    use spectral::iter::*;
    use spectral::numeric::*;
    use spectral::vec::*;

    #[test]
    fn test_remove_duplicate_keys_small() {
        let mut ks = vec![KeySpace { coords: vec![1] }, KeySpace { coords: vec![1] }];
        let manager = KeySpaceManager::new(1);
        manager.remove_duplicate_keys(&mut ks, &|k: &KeySpace| k.clone());
        assert_that!(ks.len()).is_equal_to(1);
        let mut nums = ks.iter().map(|k| k.coords[0]).collect::<Vec<i32>>();
        nums.sort();
        assert_that!(nums).is_equal_to(vec![1]);
    }

    #[test]
    fn test_remove_duplicate_keys() {
        let mut ks = vec![
            KeySpace { coords: vec![1] },
            KeySpace { coords: vec![1] },
            KeySpace { coords: vec![2] },
            KeySpace { coords: vec![4] },
            KeySpace { coords: vec![1] },
            KeySpace { coords: vec![4] },
            KeySpace { coords: vec![4] },
            KeySpace { coords: vec![6] },
            KeySpace { coords: vec![5] },
        ];
        let manager = KeySpaceManager::new(1);
        manager.remove_duplicate_keys(&mut ks, &|k: &KeySpace| k.clone());
        assert_that!(ks).has_length(5);
        let mut nums = ks.iter().map(|k| k.coords[0]).collect::<Vec<i32>>();
        nums.sort();
        assert_that!(nums).contains_all_of(&vec![&1, &2, &4, &5, &6]);
    }

    #[test]
    fn test_distance() {
        let ks = vec![
            KeySpace { coords: vec![1, 3] },
            KeySpace { coords: vec![3, 2] },
            KeySpace { coords: vec![0, 0] },
        ];
        let manager = KeySpaceManager::new(2);

        for i in 0..ks.len() {
            for j in i + 1..ks.len() {
                assert_that!(manager.distance(&ks[i], &ks[j]))
                    .is_close_to(manager.distance(&ks[j], &ks[i]), 1e-4);
            }
        }

        assert_that!(manager.distance(&ks[0], &ks[1])).is_close_to(3f32.sqrt(), 1e-4);
        assert_that!(manager.distance(&ks[0], &ks[2])).is_close_to(4f32.sqrt(), 1e-4);
        assert_that!(manager.distance(&ks[1], &ks[2])).is_close_to(5f32.sqrt(), 1e-4);
    }

    #[test]
    fn test_angle() {
        // Grid of form:
        //        |7   8   1
        //        |
        // y = 0 >|6   0   2
        //        |
        //        |5   4   3
        //         ---------
        //       x = 0 ^
        //
        // Where the numbers on the grids are the indices in `ks`
        let ks = vec![
            KeySpace { coords: vec![0, 0] },
            KeySpace { coords: vec![2, 2] },
            KeySpace { coords: vec![2, 0] },
            KeySpace {
                coords: vec![2, -2],
            },
            KeySpace {
                coords: vec![0, -2],
            },
            KeySpace {
                coords: vec![-2, -2],
            },
            KeySpace {
                coords: vec![-2, 0],
            },
            KeySpace {
                coords: vec![-2, 2],
            },
            KeySpace { coords: vec![0, 2] },
        ];
        let manager = KeySpaceManager::new(2);

        for k in &ks {
            assert_that!(manager.angle(k, k, k)).is_equal_to(0.0);
        }

        for k in &ks {
            assert_that!(manager.angle(&ks[0], k, k)).is_equal_to(0.0);
        }

        assert_that!(manager.angle(&ks[0], &ks[1], &ks[2]))
            .is_close_to(::std::f32::consts::PI / 4.0, 1e-4);
        assert_that!(manager.angle(&ks[0], &ks[2], &ks[6]))
            .is_close_to(::std::f32::consts::PI, 1e-4);
        assert_that!(manager.angle(&ks[0], &ks[3], &ks[6]))
            .is_close_to(3.0 * ::std::f32::consts::PI / 4.0, 1e-4);
        assert_that!(manager.angle(&ks[0], &ks[7], &ks[8]))
            .is_close_to(::std::f32::consts::PI / 4.0, 1e-4);
        assert_that!(manager.angle(&ks[0], &ks[3], &ks[7]))
            .is_close_to(::std::f32::consts::PI, 1e-4);
    }

    #[test]
    fn test_angle_1d() {
        let ks = vec![
            KeySpace { coords: vec![-1] },
            KeySpace { coords: vec![0] },
            KeySpace { coords: vec![1] },
            KeySpace { coords: vec![2] },
        ];
        let manager = KeySpaceManager::new(1);
        assert_that!(manager.angle(&ks[1], &ks[0], &ks[2]))
            .is_close_to(::std::f32::consts::PI, 1e-4);
        assert_that!(manager.angle(&ks[1], &ks[2], &ks[3])).is_equal_to(0.0);
    }

    #[test]
    fn test_distance_overflow() {
        use std::i32;
        use std::iter::repeat;
        let ks_size = 100;
        let manager = KeySpaceManager::new(ks_size);
        let ks1 = KeySpace {
            coords: repeat(i32::MAX).take(ks_size).collect(),
        };
        let ks2 = KeySpace {
            coords: repeat(i32::MIN).take(ks_size).collect(),
        };
        manager.distance(&ks1, &ks2);
    }

    #[test]
    fn test_dot_overflow() {
        use std::i32;
        use std::iter::repeat;
        let ks_size = 2;
        let manager = KeySpaceManager::new(ks_size);
        let ks_origin = KeySpace {
            coords: repeat(0).take(ks_size).collect(),
        };
        let ks1 = KeySpace {
            coords: repeat(i32::MAX).take(ks_size).collect(),
        };
        let ks2 = KeySpace {
            coords: repeat(i32::MIN).take(ks_size).collect(),
        };
        manager.angle(&ks_origin, &ks1, &ks2);
    }
}
