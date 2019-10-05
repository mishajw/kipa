use error::*;

use sequoia_openpgp::parse::PacketParser;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::serialize::Serialize;
use sequoia_openpgp::TPK;
use std::fmt;

/// Public key.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Key {
    /// Key ID, 8 characters long.
    pub key_id: String,
    /// Public cryptographic key data.
    pub data: Vec<u8>,
}

impl Key {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Self {
        assert_eq!(key_id.len(), 8);
        Key { key_id, data }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key({})", self.key_id)
    }
}

/// Public and private key, should not be sent anywhere.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SecretKey {
    /// Key ID, 8 characters long.
    pub key_id: String,
    /// Public and private cryptographic key data.
    data: Vec<u8>,
}

impl SecretKey {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Self {
        assert_eq!(key_id.len(), 8);
        SecretKey { key_id, data }
    }

    /// Gets the public part of the secret key.
    pub fn public(&self) -> Result<Key> {
        let tpk: TPK = PacketParser::from_bytes(&self.data)
            .and_then(TPK::from_packet_parser)
            .map_err(|e| -> Error {
                ErrorKind::GpgError("Failed to parse bytes as TPK".into(), e).into()
            })?;
        let mut public_key_data = Vec::new();
        // When we serialize TPKs, sequoia only exports the public parts. The
        // secret parts are discarded.
        tpk.serialize(&mut public_key_data).map_err(|e| -> Error {
            ErrorKind::GpgError("Failed to serialize TPK".into(), e).into()
        })?;
        Ok(Key::new(self.key_id.clone(), public_key_data))
    }

    /// Gets the secret cryptographic key data.
    pub fn secret_key_data_yes_really(&self) -> &[u8] {
        &self.data
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretKey({})", self.key_id)
    }
}
