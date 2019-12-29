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
        key_id: String,
        secret_loader: SecretLoader,
    ) -> InternalResult<SecretKey> {
        remotery_scope!("gnupg_get_local_private_key");
        trace!(
            self.log, "Requested local private key ID";
            "key_id" => &key_id);

        let secret = secret_loader.load()?;
        let key_data = self
            .get_private_key_data(&key_id, &secret)
            .map_err(InternalError::private)?;
        Ok(SecretKey::new(key_data).map_err(InternalError::private)?)
    }

    /// Gets the public key of a recipient.
    pub fn get_recipient_public_key(&self, key_id: String) -> InternalResult<Key> {
        remotery_scope!("gnupg_get_recipient_public_key");
        trace!(
            self.log, "Requested recipient public key ID";
            "key_id" => &key_id);

        let key_data = self
            .get_public_key_data(&key_id)
            .map_err(InternalError::private)?;
        // TODO: Return clear error when key doesn't exist.
        Ok(Key::new(key_data).map_err(InternalError::private)?)
    }

    /// Gets private key data from user's GnuPG directory, without passphrase.
    fn get_private_key_data(&self, key_id: &str, secret: &str) -> Result<Vec<u8>> {
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
            .chain_err(|| "Error on spawn gpg command to export key data")?;

        // Write the passphrase to stdin.
        let mut stdin = BufWriter::new(child.stdin.as_mut().chain_err(|| "Failed to get stdin")?);
        stdin
            .write(&format!("{}\n", secret).as_bytes())
            .chain_err(|| "Error on writing secret to gpg command")?;
        stdin.flush().chain_err(|| "Error on flushing gpg stdin")?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .chain_err(|| "Failed to wait for gpg and get output")?;
        self.check_output(&output)?;
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
        self.check_output(&output)?;
        // Key data is written to stdout.
        Ok(output.stdout)
    }

    /// Checks that the output of a process is healthy.
    fn check_output(&self, output: &Output) -> Result<()> {
        if !output.stderr.is_empty() {
            warn!(
                self.log, "GPG command for exporting keys printed to stderr";
                "stderr" => ::std::str::from_utf8(&output.stderr)
                    .unwrap_or("invalid utf8"));
        }
        if !output.status.success() {
            return Err(ErrorKind::CommandError("Non-successful exit code from gpg".into()).into());
        }
        if output.stdout.is_empty() {
            return Err(ErrorKind::CommandError("Nothing returned from gpg".into()).into());
        }
        Ok(())
    }
}
