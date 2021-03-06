use api::{Key, SecretKey};
use error::*;

use pgp::SecretLoader;
use slog::Logger;
use std::io::BufWriter;
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Loads GnuPG keys from the user's directory.
///
/// This struct does *not* use GPGME, but instead has a raw call to `gpg`. This is because GPGME
/// does not seem to respect the loopback pinentry mode when exporting private keys, and always
/// resorts to using the GPG agent.
///
/// Also, rust GPGME implementation doesn't seem to support static linking, which means the GPGME
/// libs must be installed on each system KIPA is deployed on.
pub struct GnupgKeyLoader {
    log: Logger,
}

impl GnupgKeyLoader {
    #[allow(missing_docs)]
    pub fn new(log: Logger) -> Self {
        GnupgKeyLoader { log }
    }

    /// Gets the user's private key.
    pub fn get_local_private_key(
        &self,
        key_name: String,
        secret_loader: SecretLoader,
    ) -> InternalResult<SecretKey> {
        remotery_scope!("gnupg_get_local_private_key");
        trace!(
            self.log, "Requested local private key ID";
            "key_name" => &key_name);

        let key_id = self.get_key_id_for_name(&key_name, true)?;
        let secret = secret_loader.load()?;
        let key_data = self.get_private_key_data(&key_id, &secret)?;
        Ok(SecretKey::new(key_data).map_err(InternalError::private)?)
    }

    /// Gets the public key of a recipient.
    pub fn get_recipient_public_key(&self, key_name: String) -> InternalResult<Key> {
        remotery_scope!("gnupg_get_recipient_public_key");
        trace!(
            self.log, "Requested recipient public key ID";
            "key_name" => &key_name);

        let key_id = self.get_key_id_for_name(&key_name, false)?;
        let key_data = self
            .get_public_key_data(&key_id)
            .map_err(InternalError::private)?;
        // TODO: Return clear error when key doesn't exist.
        Ok(Key::new(key_data).map_err(InternalError::private)?)
    }

    /// Gets private key data from user's GnuPG directory, without passphrase.
    fn get_private_key_data(&self, key_id: &str, secret: &str) -> InternalResult<Vec<u8>> {
        remotery_scope!("gpg_get_user_key_data");
        info!(
            self.log, "Spawning GPG command to export key data";
            "key_id" => key_id);

        // Spawn the GPG command.
        let mut command = Command::new("bash");
        command
            // TODO: Convert bash script to rust.
            .args(&["-c", include_str!("bash/export-secret-key.sh")])
            .args(&["--", key_id])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .chain_err(|| "Error on spawn gpg command to export key data")
            .map_err(InternalError::private)?;

        // Write the passphrase to stdin.
        let mut stdin = BufWriter::new(
            child
                .stdin
                .as_mut()
                .chain_err(|| "Failed to get stdin")
                .map_err(InternalError::private)?,
        );
        stdin
            .write(&format!("{}\n", secret).as_bytes())
            .chain_err(|| "Error on writing secret to gpg command")
            .map_err(InternalError::private)?;
        stdin
            .flush()
            .chain_err(|| "Error on flushing gpg stdin")
            .map_err(InternalError::private)?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .chain_err(|| "Failed to wait for gpg and get output")
            .map_err(InternalError::private)?;
        if output.status.code() == Some(2) {
            bail!(InternalError::public(
                "Couldn't read private key data. Is the password correct?",
                ApiErrorType::Configuration
            ));
        }
        self.check_output(&output, false)
            .map_err(InternalError::private)?;
        // Key data is written to stdout.
        Ok(output.stdout)
    }

    /// Gets public key data from user's GnuPG directory.
    fn get_public_key_data(&self, key_id: &str) -> Result<Vec<u8>> {
        let mut command = Command::new("gpg");
        command
            .args(&["--export", key_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = command
            .spawn()
            .chain_err(|| "Error on spawn gpg command to export public key data")?
            .wait_with_output()
            .chain_err(|| "Failed to wait for gpg and get output")?;
        self.check_output(&output, false)?;
        // Key data is written to stdout.
        Ok(output.stdout)
    }

    /// Gets the key ID for a key name
    ///
    /// The key name can be the email, the name of the owner, or even the key ID itself.
    fn get_key_id_for_name(&self, key_name: &str, secret_keys: bool) -> InternalResult<String> {
        let key_ids = self
            .get_all_key_ids_for_name(key_name, secret_keys)
            .map_err(InternalError::private)?;

        match key_ids.into_iter().next() {
            Some(key_id) => {
                debug!(self.log, "Resolved {} to key ID {}", key_name, key_id);
                Ok(key_id)
            }
            None => Err(InternalError::public(
                &format!("Key name {} was not found in GnuPG.", key_name),
                ApiErrorType::Configuration,
            )),
        }
    }

    /// Gets a list of public or secret key IDs.
    fn get_all_key_ids_for_name(&self, key_name: &str, secret_keys: bool) -> Result<Vec<String>> {
        let list_argument = if secret_keys {
            "--list-secret-keys"
        } else {
            "--list-keys"
        };

        let mut command = Command::new("gpg");
        command
            .args(&[list_argument, "--with-colons", key_name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = command
            .spawn()
            .chain_err(|| "Error on spawn gpg command to get key list")?
            .wait_with_output()
            .chain_err(|| "Failed to wait for gpg and get output")?;
        self.check_output(&output, true)?;

        let stdout = String::from_utf8(output.stdout)
            .chain_err(|| "Failed to pass gpg key list as UTF-8.")?;
        // Parse the --with-colons output. In this case, we're interested in the fingerprint tag,
        // and the key ID is in column 9.
        Ok(stdout
            .split('\n')
            .filter(|line| line.starts_with("fpr:"))
            .flat_map(|line| line.split(':').nth(9).into_iter())
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Checks that the output of a process is healthy.
    fn check_output(&self, output: &Output, allow_empty: bool) -> Result<()> {
        if !output.stderr.is_empty() {
            warn!(
                self.log, "GPG command for exporting keys printed to stderr";
                "stderr" => ::std::str::from_utf8(&output.stderr)
                    .unwrap_or("invalid utf8"));
        }
        if !output.status.success() {
            return Err(ErrorKind::CommandError(format!(
                "Non-successful exit code from gpg: {}",
                output
                    .status
                    .code()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "no code".into())
            ))
            .into());
        }
        if !allow_empty && output.stdout.is_empty() {
            return Err(ErrorKind::CommandError("Nothing returned from gpg".into()).into());
        }
        Ok(())
    }
}
