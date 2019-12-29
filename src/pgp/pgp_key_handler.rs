use api::{Key, SecretKey};
use error::*;

use failure;
use sequoia_openpgp;
use sequoia_openpgp::constants::SymmetricAlgorithm;
use sequoia_openpgp::crypto::KeyPair;
use sequoia_openpgp::crypto::SessionKey;
use sequoia_openpgp::packet::key::{UnspecifiedRole, UnspecifiedSecret};
use sequoia_openpgp::packet::{KeyFlags, PKESK, SKESK};
use sequoia_openpgp::parse::stream::{
    DecryptionHelper, Decryptor, MessageLayer, MessageStructure, VerificationHelper,
    VerificationResult,
};
use sequoia_openpgp::serialize::stream::{Cookie, Encryptor, LiteralWriter, Message, Signer};
use sequoia_openpgp::serialize::writer::Stack;
use sequoia_openpgp::{Fingerprint, KeyID, TPK};
use slog::Logger;
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
            "length" => data.len(), "sender" => %sender, "recipient" => %recipient);

        let mut signing_key_pair = into_keypair(&sender.secret_key_yes_really())
            .map_err(to_gpg_error("Failed to get keypair from signing key"))?;

        let encryption_recipients = recipient
            .sequoia_tpk
            .keys_valid()
            .key_flags(
                KeyFlags::default()
                    .set_encrypt_at_rest(true)
                    .set_encrypt_for_transport(true),
            )
            .map(|(_, _, key)| key.into())
            .collect::<Vec<_>>();

        let mut encrypted = Vec::new();
        {
            let stack = Message::new(&mut encrypted);
            let stack = Signer::new(stack, vec![&mut signing_key_pair], None)
                .map_err(to_gpg_error("Failed to sign data"))?;
            let stack = Encryptor::new(stack, &[], &encryption_recipients, None, None)
                .map_err(to_gpg_error("Failed to encrypt data"))?;
            write(stack, data)?;
        }
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
            "length" => data.len(), "sender" => %sender, "recipient" => %recipient);

        let gpg_helper = GpgHelper {
            sender: &sender.sequoia_tpk,
            recipient: &recipient.secret_key_yes_really(),
            log: self.log.new(o!("gpg_helper" => true)),
        };
        let mut decryptor = Decryptor::from_bytes(data, gpg_helper, None)
            .map_err(to_gpg_error("Failed to decrypt and verify data"))?;

        let mut decrypted_data = Vec::new();
        io::copy(&mut decryptor, &mut decrypted_data)
            .chain_err(|| "Failed to copy decrypted data")?;
        Ok(decrypted_data)
    }
}

/// Sequoia requires a struct that defines how to handle verification and decryption.
struct GpgHelper<'a> {
    sender: &'a TPK,
    recipient: &'a TPK,
    log: Logger,
}

impl<'a> VerificationHelper for GpgHelper<'a> {
    fn get_public_keys(&mut self, key_ids: &[KeyID]) -> sequoia_openpgp::Result<Vec<TPK>> {
        trace!(
            self.log, "Getting public key";
            "requested_key_ids" => format!("{:?}", key_ids),
            "sender_key_id" => %self.sender.primary().component().keyid(),
            "recipient_key_id" => %self.recipient.primary().component().keyid(),
        );
        Ok(key_ids
            .iter()
            .filter_map(|key_id| {
                if *key_id == self.sender.primary().component().keyid() {
                    Some(self.sender.clone())
                } else if *key_id == self.recipient.primary().component().keyid() {
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
            [VerificationResult::GoodChecksum(..)] => Ok(()),
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
        let first_session_key = pkesks
            .into_iter()
            .next()
            .ok_or(failure::err_msg("No PKESKS for decryption"))?;
        if *first_session_key.recipient() != self.recipient.keyid() {
            return Err(failure::err_msg(format!(
                "Session key was for incorrect recipient, expected {}, was {}",
                self.recipient.keyid(),
                first_session_key.recipient()
            )));
        }
        let mut pair = into_keypair(self.recipient)?;
        debug!(
            self.log, "Decrypting key"; "fingerprint" => pair.public().fingerprint().to_string());
        first_session_key
            .decrypt(&mut pair)
            .and_then(|(algo, session_key)| decrypt(algo, &session_key))
            .map(|_| None)
    }
}

fn to_gpg_error(message: &'static str) -> impl Fn(failure::Error) -> Error {
    move |error| ErrorKind::GpgError(message.into(), error).into()
}

fn write(stack: Stack<Cookie>, data: &[u8]) -> Result<()> {
    let mut literal_writer = LiteralWriter::new(
        stack,
        sequoia_openpgp::constants::DataFormat::Binary,
        None,
        None,
    )
    .map_err(to_gpg_error("Failed to init writer"))?;
    literal_writer
        .write_all(data)
        .chain_err(|| "Failed to encrypt data")?;
    literal_writer
        .finalize()
        .map_err(to_gpg_error("Failed to finalize write"))?;
    Ok(())
}

fn into_keypair(tpk: &TPK) -> sequoia_openpgp::Result<KeyPair<UnspecifiedRole>> {
    let signing_key: UnspecifiedSecret = tpk
        .keys_valid()
        .signing_capable()
        .nth(0)
        .unwrap()
        .2
        .clone()
        .into();
    signing_key.into_keypair()
}
