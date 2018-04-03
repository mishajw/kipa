//! Projects a key into an n-dimenional space in order to perform graph search.

use key::Key;

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
use std::mem::size_of;
use std::ops::{Deref, BitXor, Sub};

/// A key space value with a set of coordinates.
#[derive(Clone)]
pub struct KeySpace {
    coords: Vec<i32>
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
        let coords: Vec<i32> = chunks_transpose.iter()
            .map(|cs| {
                cs.iter()
                    .map(Deref::deref)
                    .fold(0 as u8, BitXor::bitxor)
            })
            .collect::<Vec<u8>>()
            .chunks(size_of::<i32>() / size_of::<u8>())
            .map(|cs| Cursor::new(cs)
                .read_i32::<BigEndian>().unwrap())
            .collect();
        KeySpace { coords: coords }
    }
}

impl<'a, 'b> Sub<&'b KeySpace> for &'a KeySpace {
    type Output = f32;

    fn sub(self, other: &KeySpace) -> f32 {
        assert!(self.coords.len() == other.coords.len());

        println!("{:?} and {:?}", self.coords, other.coords);

        let total: i64 = self.coords.iter()
            .zip(&other.coords)
            .map(|(a, b)| a - b)
            .fold(0 as i64, |a, b| a + (b as i64));

        (total as f32).powf(self.coords.len() as f32)
    }

}

