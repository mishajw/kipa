#[derive(Clone)]
pub struct Key {
    data: Vec<u8>
}

impl Key {
    pub fn new(data: Vec<u8>) -> Self {
        Key {data: data}
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

