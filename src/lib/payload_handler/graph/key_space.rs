//! Projects a key into an n-dimenional space in order to perform graph search.

use key::Key;

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use std::mem::size_of;
use std::ops::{BitXor, Deref, Sub};
use std::fmt;

/// A key space value with a set of coordinates.
#[derive(Clone, PartialEq)]
pub struct KeySpace {
    coords: Vec<i32>,
}

impl KeySpace {
    /// Create a location in key space from a key.
    pub fn from_key(key: &Key, size: usize) -> Self {
        let chunk_size = (size_of::<i32>() * size) / size_of::<u8>();
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

    pub fn get_size(&self) -> usize {
        self.coords.len()
    }
}

impl<'a, 'b> Sub<&'b KeySpace> for &'a KeySpace {
    type Output = f32;

    /// Gets the euclidean distance between points in key space.
    fn sub(self, other: &KeySpace) -> f32 {
        assert!(self.coords.len() == other.coords.len());

        let total: i64 = self.coords
            .iter()
            .zip(&other.coords)
            // Map to i64 so we have enough space to perform operations without
            // overflow
            .map(|(a, b)| (*a as i64, *b as i64))
            .map(|(a, b)| (a - b).abs())
            .fold(0 as i64, |a, b| a + (b as i64));

        (total as f32).powf(1f32 / self.coords.len() as f32)
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

pub fn sort_key_relative<T>(
    v: &mut Vec<T>,
    get_key_space_fn: &Fn(&T) -> KeySpace,
    key_space: &KeySpace,
) {
    // TODO: Can we use lifetimes to avoid `get_key_space_fn` returning a value,
    // and instead a reference?
    // Related: https://github.com/rust-lang/rust/issues/22340
    v.sort_by(|a: &T, b: &T| {
        let a_ks: KeySpace = get_key_space_fn(a);
        let b_ks: KeySpace = get_key_space_fn(b);
        (&a_ks - key_space)
            .partial_cmp(&(&b_ks - key_space))
            .unwrap()
    });
}

/// Remove elements from a vector that contain the same key.
pub fn remove_duplicate_keys<T>(
    v: &mut Vec<T>,
    get_key_space_fn: &Fn(&T) -> KeySpace,
) {
    if v.len() <= 1 {
        return;
    }

    let sort_key = &get_key_space_fn(&v[0]);
    sort_key_relative(v, &get_key_space_fn, sort_key);

    for i in (1..v.len()).rev() {
        let k1 = get_key_space_fn(&v[i]);
        let k2 = get_key_space_fn(&v[i - 1]);
        if k1 == k2 {
            v.swap_remove(i);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_remove_duplicate_keys_small() {
        let mut ks =
            vec![KeySpace { coords: vec![1] }, KeySpace { coords: vec![1] }];
        remove_duplicate_keys(&mut ks, &|k: &KeySpace| k.clone());
        assert_eq!(ks.len(), 1);
        let mut nums = ks.iter().map(|k| k.coords[0]).collect::<Vec<i32>>();
        nums.sort();
        assert_eq!(nums, vec![1]);
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
        remove_duplicate_keys(&mut ks, &|k: &KeySpace| k.clone());
        assert_eq!(ks.len(), 5);
        let mut nums = ks.iter().map(|k| k.coords[0]).collect::<Vec<i32>>();
        nums.sort();
        assert_eq!(nums, vec![1, 2, 4, 5, 6]);
    }

    #[test]
    fn test_distance() {
        let ks = vec![
            KeySpace { coords: vec![1, 3] },
            KeySpace { coords: vec![3, 2] },
            KeySpace { coords: vec![0, 0] },
        ];

        for i in 0..ks.len() {
            for j in i + 1..ks.len() {
                assert_eq!(&ks[i] - &ks[j], &ks[j] - &ks[i]);
            }
        }

        assert_eq!(&ks[0] - &ks[1], 3f32.sqrt());
        assert_eq!(&ks[0] - &ks[2], 4f32.sqrt());
        assert_eq!(&ks[1] - &ks[2], 5f32.sqrt());
    }
}
