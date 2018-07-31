//! Handles GPG operations using the GPGME library

use error::*;
use key::Key;

use gpgme;
use slog::Logger;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

const GNUPG_HOME_VAR: &str = "GNUPGHOME";

/// Default owned GnuPG home directory
pub const DEFAULT_OWNED_GNUPG_HOME_DIRECTORY: &str = "/tmp/kipa_gnupg";

/// Default owned GnuPG home directory
pub const DEFAULT_SECRET_PATH: &str = "./secret.txt";

/// Wrapper around the GPGME context, with operations to change the GPGME home
/// directory
///
/// There *must* be only one instance of this variable (`GPG_CONTEXT` in this
/// file), and it must be wrapped in a `Mutex`. This is because the struct can
/// set the `$GNUPGHOME` environment variable, which can not be locked. By
/// putting it in a `Mutex`, we can guarantee that the environment variable
/// will not be changed (by this code) as long as a reference to the
/// `gpgme::Context` persists. This means that `$GNUPGHOME` will not change
/// while a `gpgme::Context` is being used.
struct GpgContext {
    context: gpgme::Context,
    default_home_directory: String,
    current_home_directory: String,
}

impl GpgContext {
    fn new() -> Self {
        // Get the original GnuPG home variable
        //
        // If it doesn't exist, the original is then `~/.gnupg`.
        let default_home_directory =
            env::var(GNUPG_HOME_VAR).unwrap_or_else(|_| {
                Path::new(&env::var("HOME")
                    .expect("No home directory set in environment"))
                    .join(".gnupg")
                    .to_str()
                    .expect("Error on getting string from path")
                    .to_string()
            });
        GpgContext {
            context: gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)
                .expect("Failed to create GPGME context"),
            default_home_directory: default_home_directory.clone(),
            current_home_directory: default_home_directory,
        }
    }

    fn set_gpg_home(&mut self, home_directory: String) {
        if home_directory == self.current_home_directory {
            return;
        }
        env::set_var(GNUPG_HOME_VAR, &home_directory);
        self.current_home_directory = home_directory;
    }

    fn reset_gpg_home(&mut self) {
        let directory = self.default_home_directory.clone();
        self.set_gpg_home(directory);
    }
}

/// The wrapper is also used so that we can unsafely implement `Send` for this
/// trait, so that the same context can be used across multiple threads
///
/// TODO: Remove `unsafe impl`
unsafe impl Send for GpgContext {}

lazy_static! {
    /// The global context for GPGME
    static ref GPG_CONTEXT: Arc<Mutex<GpgContext>> =
        Arc::new(Mutex::new(GpgContext::new()));
}

/// Get the GPGME context, the home directory does not matter
macro_rules! get_context {
    ($name:ident) => {
        let $name = &mut GPG_CONTEXT.lock().unwrap().context;
    };
}

/// Get the GPGME context with the "owned" home directory, i.e. the directory
/// that we can add/remove keys to/from
macro_rules! get_owned_context {
    ($name:ident, $directory:expr) => {
        let mut wrapper = GPG_CONTEXT.lock().unwrap();
        wrapper.set_gpg_home($directory);
        let $name = &mut wrapper.context;
    };
}

/// Get the GPGME context with the "user" home directory, i.e. the directory
/// that is managed by the user and which we must not edit
macro_rules! get_user_context {
    ($name:ident) => {
        let mut wrapper = GPG_CONTEXT.lock().unwrap();
        wrapper.reset_gpg_home();
        let $name = &mut wrapper.context;
    };
}

/// Interface to GPGME functionality using library constructs
pub struct GpgKeyHandler {
    owned_gnupg_home_directory: String,
    secret: String,
    log: Logger,
}

impl GpgKeyHandler {
    /// Create a new handler. Creates a new GPGME context
    pub fn new(
        owned_gnupg_home_directory: String,
        secret_path: &str,
        log: Logger,
    ) -> InternalResult<Self>
    {
        // Create the directory for our GPG directory
        fs::create_dir_all(&owned_gnupg_home_directory).map_err(|err| {
            InternalError::public_with_error(
                &format!(
                    "Error on creating GnuPG directory at {}",
                    owned_gnupg_home_directory
                ),
                ApiErrorType::External,
                err,
            )
        })?;

        // Set trust mode to "always" for our GPG directory, so that we can use
        // imported keys
        // TODO: Better way to handle this?
        let mut gpg_conf_file =
            fs::File::create(
                Path::new(&owned_gnupg_home_directory).join("gpg.conf"),
            ).chain_err(|| "Error on creating gpg.conf file")
                .map_err(InternalError::private)?;
        gpg_conf_file
            .write_all(b"trust-model always")
            .chain_err(|| "Error on writing to gpg.conf")
            .map_err(InternalError::private)?;
        drop(gpg_conf_file);

        // Set the pinentry mode for GPG so that we can enter passphrase
        // programatically
        {
            get_context!(context);
            to_internal_result(
                context
                    .set_pinentry_mode(gpgme::PinentryMode::Loopback)
                    .chain_err(|| "Error on setting pinentry mode to loopback"),
            )?;
        }

        // Read the password from the secret file
        let mut secret_file = to_internal_result(
            fs::File::open(secret_path)
                .chain_err(|| "Error on opening secret file"),
        )?;
        let mut secret = String::new();
        to_internal_result(
            secret_file
                .read_to_string(&mut secret)
                .chain_err(|| "Error on reading secret file"),
        )?;

        debug!(log, "Created GPG key handler");
        Ok(GpgKeyHandler {
            owned_gnupg_home_directory,
            secret,
            log,
        })
    }

    /// Get the key for a key ID string. The string must be eight characters
    /// long
    pub fn get_key(&self, key_id: String) -> InternalResult<Key> {
        trace!(self.log, "Requested key ID"; "key_id" => &key_id);
        get_user_context!(context);

        let key = context.get_key(key_id.clone()).map_err(|_| {
            InternalError::public(
                &format!("Could not find key with ID {}", key_id),
                ApiErrorType::External,
            )
        })?;
        assert!(key.id().unwrap().ends_with(key_id.as_str()));
        let mut buffer = Vec::new();
        context
            .export_keys(&[key], gpgme::ExportMode::empty(), &mut buffer)
            .chain_err(|| "Error on exporting key")
            .map_err(|err| InternalError::private(err))?;

        Ok(Key::new(key_id, buffer))
    }

    /// Encrypt data for a recipient, using the recipient's public key
    pub fn encrypt(&self, data: &[u8], recipient: &Key) -> Result<Vec<u8>> {
        debug!(
            self.log, "Encrypting data";
            "length" => data.len(), "recipient" => %recipient);
        get_owned_context!(context, self.owned_gnupg_home_directory.clone());

        let gpg_key = self.ensure_key_in_gpg(recipient, context)?;
        let mut encrypted_data = Vec::new();
        context
            .encrypt(Some(&gpg_key), data, &mut encrypted_data)
            .chain_err(|| "Error on encrypt operation")?;
        debug!(
            self.log, "Encryption successful";
            "encrypted_length" => encrypted_data.len());
        Ok(encrypted_data)
    }

    /// Decrypt data from a sender, using the recipient's private key
    ///
    /// We can only decrypt with keys in the user's GPG directory.
    pub fn decrypt(&self, data: &[u8], recipient: &Key) -> Result<Vec<u8>> {
        debug!(
            self.log, "Decrypting data";
            "length" => data.len(), "recipient" => %recipient);
        get_user_context!(context);

        let mut decrypted_data = Vec::new();
        let passphrase_provider = self.get_passphrase_provider();
        context
            .with_passphrase_provider(passphrase_provider, |context| {
                context.decrypt(data, &mut decrypted_data)
            })
            .chain_err(|| "Error on decrypt operation")?;
        Ok(decrypted_data)
    }

    /// Sign data as a sender, using the sender's private key
    ///
    /// We can only sign with keys in the user's GPG directory.
    pub fn sign(&self, data: &[u8], sender: &Key) -> Result<Vec<u8>> {
        debug!(
            self.log, "Signing data";
            "length" => data.len(), "sender" => %sender);
        get_user_context!(context);

        let mut signature = Vec::new();
        let passphrase_provider = self.get_passphrase_provider();
        context.with_passphrase_provider(passphrase_provider, |context| {
            let gpg_key = context
                .get_secret_key(&sender.key_id)
                .chain_err(|| "Error on getting key for signing")?;
            context
                .add_signer(&gpg_key)
                .chain_err(|| "Error on adding signer")?;
            context
                .sign(gpgme::SignMode::Detached, data, &mut signature)
                .chain_err(|| "Error on sign operation")
        })?;
        context.clear_signers();
        debug!(
            self.log, "Signing successful";
            "signature_length" => signature.len());
        Ok(signature)
    }

    /// Verify data signed by a sender, using the sender's public key
    pub fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        sender: &Key,
    ) -> Result<()>
    {
        debug!(
            self.log, "Verifying data";
            "length" => data.len(), "sender" => %sender);
        get_owned_context!(context, self.owned_gnupg_home_directory.clone());

        let gpg_key = self.ensure_key_in_gpg(sender, context)?;

        // Get all fingerprints of the sender including subkeys, so we can check
        // if any of its subkeys signed the data
        let mut possible_fingerprints: Vec<&str> = gpg_key
            .subkeys()
            .filter_map(|k| k.fingerprint().ok())
            .collect();
        possible_fingerprints.push(&sender.key_id);

        // Get the signatures
        let signatures_result = context
            .verify_detached(signature, data)
            .chain_err(|| "Error on verifying signature")?;

        // Take the fingerprints from the keys
        let fingerprints: Vec<String> = signatures_result
            .signatures()
            .filter_map(|s| s.fingerprint().map(|fpr| fpr.to_string()).ok())
            .collect();

        // Check if any of the signature's fingerprints are any of the correct
        // fingerprints
        //
        // We use `ends_with` as sometimes key IDs are used instead of
        // fingerprints. First we check that all checked fingerprints have a
        // minimum size, so that the `ends_with` call is correct.
        for fpr in &possible_fingerprints {
            assert!(fpr.len() >= 8);
        }
        let has_found_fingerprint = fingerprints.iter().any(|fpr| {
            possible_fingerprints.iter().any(|pfpr| fpr.ends_with(pfpr))
        });

        if !has_found_fingerprint {
            return Err(ErrorKind::GpgMeError(format!(
                "Content is not signed by the correct key. Expected any of \
                 {}, found {}",
                possible_fingerprints.join(", "),
                fingerprints.join(", ")
            )).into());
        }

        Ok(())
    }

    fn ensure_key_in_gpg(
        &self,
        key: &Key,
        context: &mut gpgme::Context,
    ) -> Result<gpgme::Key>
    {
        match context.get_key(&key.key_id) {
            Ok(key) => Ok(key),
            Err(_) => {
                info!(
                    self.log, "Importing key into GPG";
                    "key_id" => &key.key_id);
                let mut key_data = gpgme::Data::from_bytes(&key.data)
                    .chain_err(|| "Error on reading key bytes")?;
                context
                    .import(&mut key_data)
                    .chain_err(|| "Error on importing key")?;
                context
                    .get_key(&key.key_id)
                    .chain_err(|| "Error on getting newly imported key")
            }
        }
    }

    fn get_passphrase_provider(
        &self,
    ) -> impl FnMut(gpgme::PassphraseRequest, &mut Write)
        -> ::std::result::Result<(), gpgme::Error> {
        let secret: Vec<u8> = self.secret.as_bytes().to_vec();
        move |_: gpgme::PassphraseRequest,
              out: &mut Write|
              -> ::std::result::Result<(), gpgme::Error> {
            out.write_all(&secret)?;
            Ok(())
        }
    }
}
