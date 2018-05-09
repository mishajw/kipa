//! Projects a key into an n-dimenional space in order to perform graph search.

use key::Key;

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use std::mem::size_of;
use std::ops::{BitXor, Deref};
use std::fmt;

/// The default dimension size for key space
pub const DEFAULT_KEY_SPACE_SIZE: usize = 2;

/// A key space value with a set of coordinates.
#[derive(Clone, PartialEq)]
pub struct KeySpace {
    coords: Vec<i32>,
}

impl KeySpace {
    pub fn get_size(&self) -> usize {
        self.coords.len()
    }
}

impl fmt::Display for KeySpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "KeySpace({})",
            self.coords
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

/// Manage how to create and compare points in `KeySpace`.
pub struct KeySpaceManager {
    num_key_space_dims: usize,
}

impl KeySpaceManager {
    /// Create a new key space manager with a key space dimensionality.
    pub fn new(num_key_space_dims: usize) -> Self {
        KeySpaceManager {
            num_key_space_dims: num_key_space_dims,
        }
    }

    /// Create a location in key space from a key.
    pub fn create_from_key(&self, key: &Key) -> KeySpace {
        let chunk_size =
            (size_of::<i32>() * self.num_key_space_dims) / size_of::<u8>();
        let chunks = key.get_data().chunks(chunk_size);
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
        KeySpace { coords: coords }
    }

    /// Gets the euclidean distance between points in key space.
    pub fn distance(&self, a_ks: &KeySpace, b_ks: &KeySpace) -> f32 {
        assert!(a_ks.coords.len() == b_ks.coords.len());

        let total: i64 = a_ks.coords
            .iter()
            .zip(&b_ks.coords)
            // Map to i64 so we have enough space to perform operations without
            // overflow
            .map(|(a, b)| (*a as i64, *b as i64))
            .map(|(a, b)| (a - b).abs())
            .fold(0 as i64, |a, b| a + (b as i64));

        (total as f32).powf(1f32 / a_ks.coords.len() as f32)
    }

    /// Get the angle between two points in key space `a` and `b`, relative to
    /// a point in key space `relative_to`.
    pub fn angle(
        &self,
        relative_to: &KeySpace,
        a: &KeySpace,
        b: &KeySpace,
    ) -> f32 {
        let dot = |a2: &KeySpace, b2: &KeySpace| -> f32 {
            let result: i64 = a2.coords
                .iter()
                .zip(&b2.coords)
                .zip(&relative_to.coords)
                .map(|((i, j), l)| ((i - l) as i64, (j - l) as i64))
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

    /// Sort a vector by each element's closeness to some key in key space.
    pub fn sort_key_relative<T>(
        &self,
        v: &mut Vec<T>,
        get_key_space_fn: &Fn(&T) -> KeySpace,
        key_space: &KeySpace,
    ) {
        // TODO: Can we use lifetimes to avoid `get_key_space_fn` returning a
        // value, and instead a reference?
        // Related: https://github.com/rust-lang/rust/issues/22340
        v.sort_by(|a: &T, b: &T| {
            let a_ks: KeySpace = get_key_space_fn(a);
            let b_ks: KeySpace = get_key_space_fn(b);
            (self.distance(&a_ks, key_space))
                .partial_cmp(&(self.distance(&b_ks, key_space)))
                .unwrap()
        });
    }

    /// Remove elements from a vector that contain the same key.
    pub fn remove_duplicate_keys<T>(
        &self,
        v: &mut Vec<T>,
        get_key_space_fn: &Fn(&T) -> KeySpace,
    ) {
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
        let mut ks =
            vec![KeySpace { coords: vec![1] }, KeySpace { coords: vec![1] }];
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

        assert_that!(manager.distance(&ks[0], &ks[1]))
            .is_close_to(3f32.sqrt(), 1e-4);
        assert_that!(manager.distance(&ks[0], &ks[2]))
            .is_close_to(4f32.sqrt(), 1e-4);
        assert_that!(manager.distance(&ks[1], &ks[2]))
            .is_close_to(5f32.sqrt(), 1e-4);
    }

    #[test]
    fn test_angle() {
        // Grid of form:
        //       |7   8   1
        //       |
        //y = 0 >|6   0   2
        //       |
        //       |5   4   3
        //       - - - - -
        //      x = 0 ^
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
}
