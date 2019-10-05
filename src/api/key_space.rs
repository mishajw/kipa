use std::fmt;

/// Location in N-dimensional "key space".
///
/// Derived from a key's public data.
#[derive(Clone, PartialEq)]
pub struct KeySpace {
    /// Coordinates in key space.
    pub coords: Vec<i32>,
}

impl KeySpace {
    #[allow(missing_docs)]
    pub fn new(coords: Vec<i32>) -> Self {
        KeySpace { coords }
    }

    #[allow(missing_docs)]
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
