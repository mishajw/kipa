use error::*;

use sequoia_openpgp::parse::PacketParser;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::serialize::Serialize;
use sequoia_openpgp::TPK;
use serde;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Public key.
#[derive(Clone)]
pub struct Key {
    /// Key ID, 8 characters long.
    pub key_id: String,
    /// The sequoia representation of the key.
    ///
    /// We leak the implementation here so we don't have to deserialize the key data on every
    /// operation.
    pub sequoia_tpk: TPK,
}

impl Key {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Result<Self> {
        assert_eq!(key_id.len(), 8);
        Ok(Key {
            key_id,
            sequoia_tpk: parse_tpk(&data)?,
        })
    }

    /// Gets the key data for serialization.
    pub fn key_data(&self) -> Vec<u8> {
        let mut public_key_data = Vec::new();
        self.sequoia_tpk
            .serialize(&mut public_key_data)
            .expect("Failed to serialize TPK");
        public_key_data
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.sequoia_tpk.fingerprint() == other.sequoia_tpk.fingerprint()
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sequoia_tpk.fingerprint().hash(state);
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key({})", self.key_id)
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.key_id)
    }
}

/// Public and private key, should not be sent anywhere.
#[derive(Clone, PartialEq)]
pub struct SecretKey {
    /// Key ID, 8 characters long.
    pub key_id: String,
    /// The sequoia representation of the key.
    ///
    /// We leak the implementation here so we don't have to deserialize the key data on every
    /// operation.
    sequoia_tpk: TPK,
}

impl SecretKey {
    #[allow(missing_docs)]
    pub fn new(key_id: String, data: Vec<u8>) -> Result<Self> {
        assert_eq!(key_id.len(), 8);
        Ok(SecretKey {
            key_id,
            sequoia_tpk: parse_tpk(&data)?,
        })
    }

    /// Gets the public part of the secret key.
    pub fn public(&self) -> Result<Key> {
        let mut public_key_data = Vec::new();
        // When we serialize TPKs, sequoia only exports the public parts. The
        // secret parts are discarded.
        self.sequoia_tpk
            .serialize(&mut public_key_data)
            .map_err(|e| -> Error {
                ErrorKind::GpgError("Failed to serialize TPK".into(), e).into()
            })?;
        Key::new(self.key_id.clone(), public_key_data)
    }

    /// Gets the secret cryptographic key data.
    pub fn secret_key_yes_really(&self) -> &TPK {
        &self.sequoia_tpk
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretKey({})", self.key_id)
    }
}

fn parse_tpk(data: &[u8]) -> Result<TPK> {
    PacketParser::from_bytes(data)
        .and_then(TPK::from_packet_parser)
        .map_err(|_| ErrorKind::ParseError("Failed to parse bytes as TPK".into()).into())
}
