use api::{Key, SecretKey};
use error::*;

use failure;
use sequoia_openpgp;
use sequoia_openpgp::constants::SymmetricAlgorithm;
use sequoia_openpgp::crypto::SessionKey;
use sequoia_openpgp::packet::{PKESK, SKESK};
use sequoia_openpgp::parse::stream::{
    DecryptionHelper, Decryptor, MessageLayer, MessageStructure,
    VerificationHelper, VerificationResult,
};
use sequoia_openpgp::parse::PacketParser;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::serialize::stream::{
    Cookie, EncryptionMode, Encryptor, LiteralWriter, Message, Signer,
};
use sequoia_openpgp::serialize::writer::Stack;
use sequoia_openpgp::{Fingerprint, KeyID, RevocationStatus, TPK};
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
        remotery_scope!("gpg_encrypt");
        debug!(
            self.log, "Encrypting data";
            "length" => data.len(), "sender" => %sender, "recipient" => %recipient);

        let recipient_tpk = key_to_tpk(&recipient.data)?;
        log_tpk(
            &recipient_tpk,
            &self.log.new(o!("encrypt" => true, "type" => "recipient")),
        );
        let sender_tpk = key_to_tpk(sender.secret_key_data_yes_really())?;
        log_tpk(
            &sender_tpk,
            &self.log.new(o!("encrypt" => true, "type" => "sender")),
        );
        let mut key_pair =
            sender_tpk.primary().clone().into_keypair().map_err(
                to_gpg_error("Failed to get keypair from signing key"),
            )?;

        let mut encrypted = Vec::new();
        {
            let stack = Message::new(&mut encrypted);
            let stack = Signer::new(stack, vec![&mut key_pair], None)
                .map_err(to_gpg_error("Failed to init signer"))?;
            let stack = Encryptor::new(
                stack,
                &[],
                &[&recipient_tpk],
                EncryptionMode::ForTransport,
                None,
            )
            .map_err(to_gpg_error("Failed to init encryptor"))?;
            write(stack, data)?;
        }
        Ok(encrypted)
    }

    /// Decrypt data for the local key, verifying it's from the sender.
    pub fn decrypt_and_sign(
        &self,
        data: &[u8],
        sender: &Key,
        recipient: &SecretKey,
    ) -> Result<Vec<u8>> {
        remotery_scope!("gpg_decrypt");
        debug!(
            self.log, "Decrypting data";
            "length" => data.len(), "sender" => %sender, "recipient" => %recipient);

        let sender_tpk = key_to_tpk(&sender.data)?;
        log_tpk(
            &sender_tpk,
            &self.log.new(o!("decrypt" => true, "type" => "sender")),
        );
        let recipient_tpk = key_to_tpk(recipient.secret_key_data_yes_really())?;
        log_tpk(
            &recipient_tpk,
            &self.log.new(o!("decrypt" => true, "type" => "recipient")),
        );
        let helper = GpgHelper {
            sender: &sender_tpk,
            recipient: &recipient_tpk,
            log: self.log.new(o!("helper" => true)),
        };
        let mut decryptor = Decryptor::from_bytes(data, helper, None)
            .map_err(to_gpg_error("Failed to init decryptor"))?;

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
    fn get_public_keys(
        &mut self,
        key_ids: &[KeyID],
    ) -> sequoia_openpgp::Result<Vec<TPK>> {
        trace!(
            self.log, "Getting public key";
            "requested_key_ids" => format!("{:?}", key_ids),
            "sender_key_id" => %self.sender.primary().keyid(),
            "recipient_key_id" => %self.recipient.primary().keyid(),
        );
        return Ok(key_ids
            .into_iter()
            .filter_map(|key_id| {
                if *key_id == self.sender.primary().keyid() {
                    Some(self.sender.clone())
                } else if *key_id == self.recipient.primary().keyid() {
                    Some(self.recipient.clone())
                } else {
                    None
                }
            })
            .collect());
    }

    fn check(
        &mut self,
        structure: &MessageStructure,
    ) -> sequoia_openpgp::Result<()> {
        info!(self.log, "Checking signature");
        // We sign first, so we take the first layer of the structure.
        let verification_results = match structure.iter().next() {
            Some(MessageLayer::SignatureGroup { results }) => Ok(results),
            Some(_) => Err(failure::err_msg("Non-sig layer in structure")),
            None => Err(failure::err_msg("No layers found in structure")),
        }?;
        match &verification_results[..] {
            &[VerificationResult::GoodChecksum(..)] => Ok(()),
            &[_] => Err(failure::err_msg("Bad verification result")),
            &[] => Err(failure::err_msg("No verification results")),
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
        D: FnMut(
            SymmetricAlgorithm,
            &SessionKey,
        ) -> sequoia_openpgp::Result<()>,
    {
        let key = self.recipient.primary().clone();
        debug!(self.log, "Decrypting key"; "fingerprint" => key.fingerprint().to_string());
        let mut pair = key.into_keypair()?;
        pkesks[0]
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

fn key_to_tpk(key_data: &[u8]) -> Result<TPK> {
    // TODO: This function gets called on every encr/decr operation. This isn't
    // too slow, but still should be fixed.
    remotery_scope!("gpg_parse_tpk");
    PacketParser::from_bytes(&key_data)
        .and_then(TPK::from_packet_parser)
        .map_err(|_| {
            ErrorKind::ParseError("Failed to parse bytes as TPK".into()).into()
        })
}

fn log_tpk(tpk: &TPK, log: &Logger) {
    remotery_scope!("gpg_log_tpk");
    let log = log.new(o!(
        "tpk" => true,
        "fingerprint" => tpk.fingerprint().to_string(),
        "is_tsk" => tpk.is_tsk(),
    ));
    trace!(log, "Printing TPK");
    trace!(
        log, "Found primary key";
        "fingerprint" => tpk.primary().fingerprint().to_string(),
    );
    for subkey_bindings in tpk.subkeys() {
        trace!(
            log, "Found subkey";
            "fingerprint" => subkey_bindings.subkey().fingerprint().to_string(),
        );
    }
    for (_, revoked, key) in tpk.keys_all().unfiltered() {
        let secret = key.secret();
        let keypair = key.clone().into_keypair();
        trace!(
            log, "Found key in all keys";
            "fingerprint" => key.fingerprint().to_string(),
            "revoked" => revoked != RevocationStatus::NotAsFarAsWeKnow,
            "has_secret" => secret.is_some(),
            "has_keypair" => keypair.is_ok(),
        );
        if let Some(secret) = secret {
            trace!(
                log, "Found secret key";
                "encrypted" => secret.is_encrypted(),
            );
        }
        if let Ok(keypair) = keypair {
            trace!(
                log, "Found keypair";
                "public_fingerprint" => keypair.public().fingerprint().to_string(),
                "public_has_secret" => keypair.public().secret().is_some(),
                "public_has_keypair" => keypair.public().clone().into_keypair().is_ok(),
            );
        }
    }
}
