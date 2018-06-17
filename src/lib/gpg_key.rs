//! Manages GPG keys using GPGME.

use api::ApiError;
use error::*;
use key::Key;

use gpgme;
use slog::Logger;

/// Provide wrapped GPGME functionality.
pub struct GpgKeyHandler {
    context: gpgme::Context,
    log: Logger,
}

impl GpgKeyHandler {
    /// Create a new handler. Creates a new GPGME context.
    pub fn new(log: Logger) -> InternalResult<Self> {
        let context = to_internal_result(
            gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)
                .chain_err(|| "Error on creating GPGME context"),
        )?;
        debug!(log, "Created GPG key handler");
        Ok(GpgKeyHandler { context, log })
    }

    /// Get the key for a key ID string. The string must be eight characters
    /// long.
    pub fn get_key(&mut self, key_id: String) -> InternalResult<Key> {
        trace!(self.log, "Requested key ID"; "key_id" => &key_id);

        let key = self.context.find_key(key_id.clone()).map_err(|_| {
            InternalError::PublicError(ApiError::new(
                "Error on finding key".into(),
            ))
        })?;
        assert!(key.id().unwrap().ends_with(key_id.as_str()));
        let mut buffer = Vec::new();
        self.context
            .export_keys(&[key], gpgme::ExportMode::empty(), &mut buffer)
            .map_err(|_| {
                InternalError::private(ErrorKind::GpgMeError(
                    "Error on exporting key".into(),
                ))
            })?;

        Ok(Key::new(key_id, buffer))
    }
}
