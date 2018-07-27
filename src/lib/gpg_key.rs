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
struct GpgContext {
    context: gpgme::Context,
}

impl GpgContext {
    /// The only instance of `GpgContext` is a global static `Mutex`, meaning
    /// that the environment variable changes will not have read/write races.
    fn set_gpg_home(&self, home_directory: &str) {
        env::set_var(GNUPG_HOME_VAR, home_directory)
    }
}

/// The wrapper is also used so  that we can unsafely implement `Send` for this
/// trait, so that the same context can be used across multiple threads.
///
/// TODO: Remove `unsafe impl`
unsafe impl Send for GpgContext {}

lazy_static! {
    /// The global context for GPGME
    static ref GPG_CONTEXT: Arc<Mutex<GpgContext>> =
        Arc::new(Mutex::new(GpgContext {
            context: gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)
                .expect("Failed to create GPGME context")
        }));
}

/// Macro to handle locking and unwrapping the GPGME context
macro_rules! get_context {
    () => {
        &mut GPG_CONTEXT.lock().unwrap().context
    };
}

/// Interface to GPGME functionality using library constructs
pub struct GpgKeyHandler {
    owned_gnupg_home_directory: String,
    user_gnupg_home_directory: String,
    active_directory_type: Mutex<GpgDirectoryType>,
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
        // Store the original GPG directory
        let default_gnupg_home_directory = Path::new(
            &env::var("HOME").expect("No home directory set in environment")
        ).join(".gnupg")
            .to_str()
            .expect("Error on getting string from path")
            .to_string();
        let user_gnupg_home_directory =
            env::var(GNUPG_HOME_VAR).unwrap_or(default_gnupg_home_directory);

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
        to_internal_result(
            get_context!()
                .set_pinentry_mode(gpgme::PinentryMode::Loopback)
                .chain_err(|| "Error on setting pinentry mode to loopback"),
        )?;

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
            user_gnupg_home_directory,
            active_directory_type: Mutex::new(GpgDirectoryType::UserDirectory),
            secret,
            log,
        })
    }

    /// Get the key for a key ID string. The string must be eight characters
    /// long
    pub fn get_key(&self, key_id: String) -> InternalResult<Key> {
        trace!(self.log, "Requested key ID"; "key_id" => &key_id);
        // Ensure we are using the user's GPG directory
        self.switch_directory_type(GpgDirectoryType::UserDirectory);
        let context = get_context!();

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
        self.switch_directory_type(GpgDirectoryType::OwnedDirectory);
        let context = get_context!();

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
        self.switch_directory_type(GpgDirectoryType::UserDirectory);
        let context = get_context!();

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
        self.switch_directory_type(GpgDirectoryType::UserDirectory);
        let context = get_context!();

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
        self.switch_directory_type(GpgDirectoryType::OwnedDirectory);
        let context = get_context!();

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

    fn switch_directory_type(&self, directory_type: GpgDirectoryType) {
        let active_directory_type: &mut GpgDirectoryType =
            &mut self.active_directory_type.lock().unwrap();

        // If we're already in the correct directory, don't do anything
        if *active_directory_type == directory_type {
            return;
        }

        match directory_type {
            GpgDirectoryType::UserDirectory => {
                debug!(
                    self.log, "Changing to user GPG home directory";
                    "directory" => self.user_gnupg_home_directory.clone());
                GPG_CONTEXT
                    .lock()
                    .unwrap()
                    .set_gpg_home(&self.user_gnupg_home_directory)
            }
            GpgDirectoryType::OwnedDirectory => {
                debug!(
                    self.log, "Changing to owned GPG home directory";
                    "directory" => self.owned_gnupg_home_directory.clone());
                GPG_CONTEXT
                    .lock()
                    .unwrap()
                    .set_gpg_home(&self.owned_gnupg_home_directory)
            }
        };

        *active_directory_type = directory_type;
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

/// The type of a GPG directory
#[derive(PartialEq, Eq)]
enum GpgDirectoryType {
    /// Directory owned by a user, should not be modified
    UserDirectory,
    /// Directory owned by us, can be modified by adding new keys
    OwnedDirectory,
}
