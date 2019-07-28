//! Handles GPG operations using the GPGME library

use api::Key;
use error::*;

use gpgme;
use slog::Logger;
use std::cell::{RefCell, RefMut};
use std::env;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const GNUPG_HOME_VAR: &str = "GNUPGHOME";

/// Default owned GnuPG home directory
pub const DEFAULT_OWNED_GNUPG_HOME_DIRECTORY: &str = "/tmp/kipa_gnupg";

/// Default owned GnuPG home directory
pub const DEFAULT_SECRET_PATH: &str = "./secret.txt";

thread_local! {
    static CONTEXT: RefCell<gpgme::Context> = RefCell::new({
        let mut context = gpgme::Context::from_protocol(
            gpgme::Protocol::OpenPgp
        ).expect("Failed to create GPGME context");
        context.set_pinentry_mode(gpgme::PinentryMode::Loopback)
            .expect("Error on setting pinentry mode to loopback");
        context
    });
}

fn with_context<T>(callback: impl FnOnce(RefMut<gpgme::Context>) -> T) -> T {
    CONTEXT.with(|c| callback(c.borrow_mut()))
}

fn with_secret_context<T>(
    secret: &str,
    callback: impl FnOnce(&mut gpgme::Context) -> T,
) -> T {
    // Turn the secret into bytes
    let secret: Vec<u8> = secret.as_bytes().to_vec();

    // Passphrase provider returns the secret
    let passphrase_provider =
        move |_: gpgme::PassphraseRequest,
              out: &mut Write|
              -> ::std::result::Result<(), gpgme::Error> {
            out.write_all(&secret)?;
            Ok(())
        };

    with_context(|mut c| {
        c.with_passphrase_provider(passphrase_provider, callback)
    })
}

/// Interface to GPGME functionality using library constructs
pub struct GpgKeyHandler {
    secret: String,
    user_gpg_home_directory: Option<String>,
    log: Logger,
}

impl GpgKeyHandler {
    /// Create a new handler. Creates a new GPGME context
    pub fn new(
        owned_gpg_home_directory: &str,
        secret_path: &str,
        log: Logger,
    ) -> InternalResult<Self> {
        Self::create_owned_directory(owned_gpg_home_directory)?;
        let secret = Self::get_secret(secret_path)?;

        // Manage the GPG directories
        let user_gpg_home_directory = env::var(GNUPG_HOME_VAR).ok();
        env::set_var(GNUPG_HOME_VAR, owned_gpg_home_directory);

        Ok(GpgKeyHandler {
            secret,
            user_gpg_home_directory,
            log,
        })
    }

    /// Create the GPG directory that is modified by us
    fn create_owned_directory(directory: &str) -> InternalResult<()> {
        // Create the directory for our GPG directory
        fs::create_dir_all(directory).map_err(|err| {
            InternalError::public_with_error(
                &format!("Error on creating GnuPG directory at {}", directory),
                ApiErrorType::External,
                err,
            )
        })?;

        // Set trust mode to "always" for our GPG directory, so that we can use
        // imported keys
        let mut gpg_conf_file =
            fs::File::create(Path::new(directory).join("gpg.conf"))
                .chain_err(|| "Error on creating gpg.conf file")
                .map_err(InternalError::private)?;
        gpg_conf_file
            .write_all(b"trust-model always")
            .chain_err(|| "Error on writing to gpg.conf")
            .map_err(InternalError::private)?;
        Ok(())
    }

    /// Copy user's key from their GPG directory to our GPG directory
    pub fn copy_user_key(
        &self,
        key_id: &str,
        is_secret: bool,
    ) -> InternalResult<()> {
        remotery_scope!("gpg_copy_user_key");

        info!(
            self.log, "Copying user's key into owned GPG directory";
            "key_id" => key_id,
            "is_secret" => is_secret);

        // Function for checking if the key is imported
        let check_key_imported = || -> bool {
            if is_secret {
                with_context(|mut c| c.get_secret_key(key_id)).is_ok()
            } else {
                with_context(|mut c| c.get_key(key_id)).is_ok()
            }
        };

        // Check if the key has already been moved
        if check_key_imported() {
            return Ok(());
        }

        // Import the key data from the user's directory
        let user_key_data = self.get_user_key_data(key_id, is_secret)?;
        with_context(|mut c| c.import(user_key_data))
            .chain_err(|| "Error on importing user's key into owned directory")
            .map_err(|err| InternalError::private(err))?;

        debug_assert!(check_key_imported());
        Ok(())
    }

    /// Get key data from the user's GPG directory
    ///
    /// TODO: This function does *not* use GPGME, but instead has a raw call to
    /// `gpg`. This is because GPGME does not seem to respect the loopback
    /// pinentry mode when exporting private keys, and always resorts to
    /// using the GPG agent.
    fn get_user_key_data(
        &self,
        key_id: &str,
        is_secret: bool,
    ) -> InternalResult<Vec<u8>> {
        remotery_scope!("gpg_get_user_key_data");

        info!(
            self.log, "Spawning GPG command to export key data";
            "key_id" => key_id,
            "is_secret" => is_secret);

        let export_option = if is_secret {
            "--export-secret-keys"
        } else {
            "--export"
        };

        // Spawn the GPG command
        let mut gpg_command = Command::new("gpg");
        gpg_command
            // `pinentry-mode` and `passphrase-fd` allow us to write the
            // passphrase to stdin
            .args(&["--pinentry-mode", "loopback"])
            .args(&["--passphrase-fd", "0"])
            .args(&[export_option, key_id]);
        gpg_command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Use the user's gpg home directory
        match &self.user_gpg_home_directory {
            Some(directory) => gpg_command.env(GNUPG_HOME_VAR, directory),
            None => gpg_command.env_remove(GNUPG_HOME_VAR),
        };

        let gpg_child = gpg_command
            .spawn()
            .chain_err(|| "Error on spawn gpg command to export key data")
            .map_err(InternalError::private)?;

        // Capture stdin/stdout/stderr
        let mut gpg_stdin = BufWriter::new(
            gpg_child
                .stdin
                .chain_err(|| "Error on get gpg command's stdin")
                .map_err(InternalError::private)?,
        );
        let mut gpg_stdout = BufReader::new(
            gpg_child
                .stdout
                .chain_err(|| "Error on get gpg command's stdout")
                .map_err(InternalError::private)?,
        );
        let mut gpg_stderr = BufReader::new(
            gpg_child
                .stderr
                .chain_err(|| "Error on get gpg command's stderr")
                .map_err(InternalError::private)?,
        );

        // Write the passphrase to stdin
        gpg_stdin
            .write(&format!("{}\n", self.secret).as_bytes())
            .chain_err(|| "Error on writing secret to gpg command")
            .map_err(InternalError::private)?;
        gpg_stdin
            .flush()
            .chain_err(|| "Error on flushing gpg stdin")
            .map_err(InternalError::private)?;

        // Read the key data from stdout
        let mut key_data = Vec::new();
        gpg_stdout
            .read_to_end(&mut key_data)
            .chain_err(|| "Error on reading key data from gpg command")
            .map_err(InternalError::private)?;

        // Check if anything was printed to stderr
        let mut stderr_logs = Vec::new();
        gpg_stderr
            .read_to_end(&mut stderr_logs)
            .chain_err(|| "Error on reading stderr from gpg command")
            .map_err(InternalError::private)?;
        if stderr_logs.len() > 0 {
            warn!(
                self.log, "GPG command for exporting keys printed to stderr";
                "stderr" => ::std::str::from_utf8(&stderr_logs)
                    .unwrap_or("invalid utf8"));
        }

        Ok(key_data)
    }

    /// Get the password for the user's GPG key
    pub fn get_secret(secret_path: &str) -> InternalResult<String> {
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

        Ok(secret)
    }

    /// Get the key for a key ID string. The string must be eight characters
    /// long
    pub fn get_user_key(&self, key_id: String) -> InternalResult<Key> {
        remotery_scope!("gpg_get_user_key");

        trace!(self.log, "Requested key ID"; "key_id" => &key_id);

        // Copy the key into the owned directory
        self.copy_user_key(&key_id, false)?;

        // Get the key from the owned directory
        let key = with_context(|mut c| c.get_key(&key_id)).map_err(|_| {
            InternalError::public(
                &format!("Could not find key with ID {}", key_id),
                ApiErrorType::External,
            )
        })?;

        // Check the key id is correct
        assert!(key.id().unwrap().ends_with(key_id.as_str()));

        // Get the key data
        let mut buffer = Vec::new();
        with_context(|mut c| {
            c.export_keys(&[key], gpgme::ExportMode::empty(), &mut buffer)
        })
        .chain_err(|| "Error on exporting key")
        .map_err(|err| InternalError::private(err))?;

        Ok(Key::new(key_id, buffer))
    }

    /// Encrypt data for a recipient, using the recipient's public key
    pub fn encrypt(&self, data: &[u8], recipient: &Key) -> Result<Vec<u8>> {
        remotery_scope!("gpg_encrypt");

        debug!(
            self.log, "Encrypting data";
            "length" => data.len(), "recipient" => %recipient);

        let gpg_key = self.ensure_key_in_gpg(recipient)?;
        let mut encrypted_data = Vec::new();
        with_context(|mut c| {
            c.encrypt(Some(&gpg_key), data, &mut encrypted_data)
        })
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
        remotery_scope!("gpg_decrypt");

        debug!(
            self.log, "Decrypting data";
            "length" => data.len(), "recipient" => %recipient);
        let mut decrypted_data = Vec::new();
        with_secret_context(&self.secret, |c| {
            c.decrypt(data, &mut decrypted_data)
        })
        .chain_err(|| "Error on decrypt operation")?;
        Ok(decrypted_data)
    }

    /// Sign data as a sender, using the sender's private key
    ///
    /// We can only sign with keys in the user's GPG directory.
    pub fn sign(&self, data: &[u8], sender: &Key) -> Result<Vec<u8>> {
        remotery_scope!("gpg_sign");

        debug!(
            self.log, "Signing data";
            "length" => data.len(), "sender" => %sender);

        let mut signature = Vec::new();
        with_secret_context(&self.secret, |c| {
            let gpg_key = c
                .get_secret_key(&sender.key_id)
                .chain_err(|| "Error on getting key for signing")?;
            c.add_signer(&gpg_key)
                .chain_err(|| "Error on adding signer")?;
            c.sign(gpgme::SignMode::Detached, data, &mut signature)
                .chain_err(|| "Error on sign operation")
        })?;
        with_context(|mut c| c.clear_signers());
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
    ) -> Result<()> {
        remotery_scope!("gpg_verify");

        debug!(
            self.log, "Verifying data";
            "length" => data.len(), "sender" => %sender);
        let gpg_key = self.ensure_key_in_gpg(sender)?;

        // Get all fingerprints of the sender including subkeys, so we can check
        // if any of its subkeys signed the data
        let mut possible_fingerprints: Vec<&str> = gpg_key
            .subkeys()
            .filter_map(|k| k.fingerprint().ok())
            .collect();
        possible_fingerprints.push(&sender.key_id);

        // Get the signatures
        let signatures_result =
            with_context(|mut c| c.verify_detached(signature, data))
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
            ))
            .into());
        }

        Ok(())
    }

    fn ensure_key_in_gpg(&self, key: &Key) -> Result<gpgme::Key> {
        remotery_scope!("gpg_ensure_key_in_gpg");

        match with_context(|mut c| c.get_key(&key.key_id)) {
            Ok(key) => Ok(key),
            Err(_) => {
                info!(
                    self.log, "Importing key into GPG";
                    "key_id" => &key.key_id);
                let mut key_data = gpgme::Data::from_bytes(&key.data)
                    .chain_err(|| "Error on reading key bytes")?;
                with_context(|mut c| {
                    {
                        remotery_scope!("gpg_import_key");
                        c.import(&mut key_data)
                            .chain_err(|| "Error on importing key")?;
                    }
                    {
                        remotery_scope!("gpg_check_key_imported");
                        c.get_key(&key.key_id)
                            .chain_err(|| "Error on getting newly imported key")
                    }
                })
            }
        }
    }
}
