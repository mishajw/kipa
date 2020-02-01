use error::*;

use sequoia_openpgp::parse::PacketParser;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::serialize::Serialize;
use sequoia_openpgp::Cert;
use serde;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Public key.
#[derive(Clone)]
pub struct Key {
    /// The sequoia representation of the key.
    ///
    /// We leak the implementation here so we don't have to deserialize the key data on every
    /// operation.
    pub sequoia_cert: Cert,
}

impl Key {
    #[allow(missing_docs)]
    pub fn new(data: Vec<u8>) -> Result<Self> {
        Ok(Key {
            sequoia_cert: parse_cert(&data)?,
        })
    }

    /// Gets the key data for serialization.
    pub fn key_data(&self) -> Vec<u8> {
        let mut public_key_data = Vec::new();
        self.sequoia_cert
            .serialize(&mut public_key_data)
            .expect("Failed to serialize Cert");
        public_key_data
    }

    /// Gets the key ID string (8 characters long).
    pub fn key_id(&self) -> String {
        let sequoia_key_id = self.sequoia_cert.keyid().to_hex();
        assert_eq!(sequoia_key_id.len(), 16);
        sequoia_key_id[8..].to_string()
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.sequoia_cert.fingerprint() == other.sequoia_cert.fingerprint()
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sequoia_cert.fingerprint().hash(state);
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Key({})", self.key_id())
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.key_id())
    }
}

/// Public and private key, should not be sent anywhere.
#[derive(Clone, PartialEq)]
pub struct SecretKey {
    /// The sequoia representation of the key.
    ///
    /// We leak the implementation here so we don't have to deserialize the key data on every
    /// operation.
    sequoia_cert: Cert,
}

impl SecretKey {
    #[allow(missing_docs)]
    pub fn new(data: Vec<u8>) -> Result<Self> {
        let cert = parse_cert(&data)?;
        if !cert.is_tsk() {
            bail!(ErrorKind::ConfigError("Provided key is not a secret key".into()));
        }
        Ok(SecretKey {
            sequoia_cert: cert,
        })
    }

    /// Gets the public part of the secret key.
    pub fn public(&self) -> Result<Key> {
        let mut public_key_data = Vec::new();
        // When we serialize certificates, sequoia only exports the public parts. The
        // secret parts are discarded.
        self.sequoia_cert
            .serialize(&mut public_key_data)
            .map_err(|e| -> Error {
                ErrorKind::GpgError("Failed to serialize certificate".into(), e).into()
            })?;
        Key::new(public_key_data)
    }

    /// Gets the secret cryptographic key data.
    pub fn secret_key_yes_really(&self) -> &Cert {
        &self.sequoia_cert
    }

    /// Gets the key ID string (8 characters long).
    pub fn key_id(&self) -> String {
        let sequoia_key_id = self.sequoia_cert.keyid().to_hex();
        assert_eq!(sequoia_key_id.len(), 16);
        sequoia_key_id[8..].to_string()
    }
}

impl fmt::Display for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SecretKey({})", self.key_id())
    }
}

fn parse_cert(data: &[u8]) -> Result<Cert> {
    PacketParser::from_bytes(data)
        .and_then(Cert::from_packet_parser)
        .map_err(|e| ErrorKind::GpgError("Failed to parse bytes as certificate".into(), e).into())
}
