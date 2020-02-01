use api::{Key, SecretKey};
use error::*;

use failure;
use sequoia_openpgp;
use sequoia_openpgp::crypto::SessionKey;
use sequoia_openpgp::packet::{PKESK, SKESK};
use sequoia_openpgp::parse::stream::{
    DecryptionHelper, Decryptor, MessageLayer, MessageStructure, VerificationHelper,
    VerificationResult,
};
use sequoia_openpgp::serialize::stream::{Cookie, Encryptor, LiteralWriter, Message, Signer};
use sequoia_openpgp::serialize::writer::Stack;
use sequoia_openpgp::types::SymmetricAlgorithm;
use sequoia_openpgp::{Cert, Fingerprint, KeyHandle};
use slog::Logger;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::io;
use std::io::Write;

/// Handles encryption/decryption to/from different keys.
pub struct PgpKeyHandler {
    log: Logger,
}

impl PgpKeyHandler {
    #[allow(missing_docs)]
    pub fn new(log: Logger) -> Self {
        PgpKeyHandler { log }
    }

    /// Encrypt data for a recipient, signed by the local key.
    pub fn encrypt_and_sign(
        &self,
        data: &[u8],
        sender: &SecretKey,
        recipient: &Key,
    ) -> Result<Vec<u8>> {
        remotery_scope!("gpg_encrypt_and_sign");
        debug!(
            self.log, "Encrypting and signing data";
            "sender" => %sender,
            "recipient" => %recipient);
        log_key(
            &self.log,
            "Encryption sender",
            &sender.secret_key_yes_really(),
        );
        log_key(&self.log, "Encryption recipient", &recipient.sequoia_cert);
        log_data(&self.log, "Data before encryption", data);

        let signing_key_pair = sender
            .secret_key_yes_really()
            .keys()
            .alive()
            .revoked(false)
            .for_signing()
            .nth(0)
            .ok_or(ErrorKind::ConfigError("No signing key found.".into()))?
            .key()
            .clone()
            .mark_parts_secret()
            .map_err(to_gpg_error("Failed to mark parts as secret"))?
            .into_keypair()
            .map_err(to_gpg_error("Failed to convert to key pair"))?;

        let mut encryption_recipients = recipient
            .sequoia_cert
            .keys()
            .alive()
            .revoked(false)
            .for_transport_encryption()
            .map(|ka| ka.key().into())
            .collect::<Vec<_>>();

        let mut encrypted = Vec::new();
        {
            let stack = Message::new(&mut encrypted);
            let stack = Signer::new(stack, signing_key_pair)
                .build()
                .map_err(to_gpg_error("Failed to sign data"))?;
            let mut encryptor = Encryptor::for_recipient(
                stack,
                encryption_recipients
                    .pop()
                    .chain_err(|| "No suitable recipient keys found")?,
            );
            for recipient in encryption_recipients {
                encryptor = encryptor.add_recipient(recipient);
            }
            let stack = encryptor
                .build()
                .map_err(to_gpg_error("Failed to encrypt data"))?;
            write(stack, data)?;
        }
        debug!(
            self.log, "Finished encrypting and signing data";
            "sender" => %sender,
            "recipient" => %recipient);
        log_data(&self.log, "Data after encryption", &encrypted);
        Ok(encrypted)
    }

    /// Decrypt data for the local key, verifying it's from the sender.
    pub fn decrypt_and_verify(
        &self,
        data: &[u8],
        sender: &Key,
        recipient: &SecretKey,
    ) -> Result<Vec<u8>> {
        remotery_scope!("gpg_decrypt_and_verify");
        debug!(
            self.log, "Decrypting and verifying data";
            "sender" => %sender,
            "recipient" => %recipient);
        log_key(&self.log, "Decryption sender", &sender.sequoia_cert);
        log_key(
            &self.log,
            "Decryption recipient",
            &recipient.secret_key_yes_really(),
        );
        log_data(&self.log, "Data before decryption", data);

        let gpg_helper = GpgHelper {
            sender: &sender.sequoia_cert,
            recipient: &recipient.secret_key_yes_really(),
            log: self.log.new(o!("gpg_helper" => true)),
        };
        let mut decryptor = Decryptor::from_bytes(data, gpg_helper, None)
            .map_err(to_gpg_error("Failed to decrypt and verify data"))?;

        let mut decrypted = Vec::new();
        io::copy(&mut decryptor, &mut decrypted).chain_err(|| "Failed to copy decrypted data")?;

        debug!(
            self.log, "Finished decrypting and verifying data";
            "sender" => %sender,
            "recipient" => %recipient);
        log_data(&self.log, "Data after decryption", &decrypted);
        Ok(decrypted)
    }
}

/// Sequoia requires a struct that defines how to handle verification and decryption.
struct GpgHelper<'a> {
    sender: &'a Cert,
    recipient: &'a Cert,
    log: Logger,
}

impl<'a> VerificationHelper for GpgHelper<'a> {
    fn get_public_keys(&mut self, key_ids: &[KeyHandle]) -> sequoia_openpgp::Result<Vec<Cert>> {
        trace!(
            self.log, "Getting public key";
            "requested_key_ids" => format!("{:?}", key_ids),
            "sender_key_id" => %self.sender.primary().keyid(),
            "recipient_key_id" => %self.recipient.primary().keyid(),
        );
        Ok(key_ids
            .iter()
            .filter_map(|key_id| {
                if *key_id == self.sender.primary().keyid().into() {
                    Some(self.sender.clone())
                } else if *key_id == self.recipient.primary().keyid().into() {
                    Some(self.recipient.clone())
                } else {
                    None
                }
            })
            .collect())
    }

    fn check(&mut self, structure: &MessageStructure) -> sequoia_openpgp::Result<()> {
        info!(self.log, "Checking signature");
        // We sign first, so we take the first layer of the structure.
        let verification_results = match structure.iter().next() {
            Some(MessageLayer::SignatureGroup { results }) => Ok(results),
            Some(_) => Err(failure::err_msg("Non-sig layer in structure")),
            None => Err(failure::err_msg("No layers found in structure")),
        }?;
        match verification_results[..] {
            [VerificationResult::GoodChecksum { .. }] => Ok(()),
            [_] => Err(failure::err_msg("Bad verification result")),
            [] => Err(failure::err_msg("No verification results")),
            _ => Err(failure::err_msg("Multiple verification results")),
        }
    }
}

impl<'a> DecryptionHelper for GpgHelper<'a> {
    fn decrypt<D>(
        &mut self,
        pkesks: &[PKESK],
        _skesks: &[SKESK],
        mut decrypt: D,
    ) -> sequoia_openpgp::Result<Option<Fingerprint>>
    where
        D: FnMut(SymmetricAlgorithm, &SessionKey) -> sequoia_openpgp::Result<()>,
    {
        let (pkesk, subkey) = pkesks
            .into_iter()
            .flat_map(|pkesk| self.recipient.subkeys().map(move |subkey| (pkesk, subkey)))
            .filter(|(pkesk, subkey)| *pkesk.recipient() == subkey.component().keyid())
            .next()
            .ok_or(failure::err_msg("No PKESKs matched recipient subkeys."))?;
        let mut keypair = subkey.key().clone().mark_parts_secret()?.into_keypair()?;
        pkesk
            .decrypt(&mut keypair)
            .and_then(|(algo, session_key)| decrypt(algo, &session_key))
            .map(|_| None)
    }
}

fn to_gpg_error(message: &'static str) -> impl Fn(failure::Error) -> Error {
    move |error| ErrorKind::GpgError(message.into(), error).into()
}

fn write(stack: Stack<Cookie>, data: &[u8]) -> Result<()> {
    let mut literal_writer = LiteralWriter::new(stack)
        .build()
        .map_err(to_gpg_error("Failed to init writer"))?;
    literal_writer
        .write_all(data)
        .chain_err(|| "Failed to encrypt data")?;
    literal_writer
        .finalize()
        .map_err(to_gpg_error("Failed to finalize write"))?;
    Ok(())
}

fn log_key(log: &Logger, message: &str, key: &Cert) {
    let subkey_fingerprints: Vec<String> = key
        .subkeys()
        .map(|key| key.component().fingerprint().to_hex())
        .collect();
    trace!(
        log, "{}", message;
        "key_id" => key.keyid().to_hex(),
        "fingerprint" => key.fingerprint().to_hex(),
        "primary_fingerprint" => key.primary().fingerprint().to_hex(),
        "subkey_fingerprints" => subkey_fingerprints.join(", "));
}

fn log_data(log: &Logger, message: &str, data: &[u8]) {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();
    trace!(log, "{}", message; "length" => data.len(), "hash" => hash);
}
