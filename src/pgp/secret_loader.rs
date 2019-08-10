use error::*;

use slog::Logger;
use std::fs;
use std::io::Read;

/// Loads user's secret password.
pub struct SecretLoader {
    secret_path: String,
    log: Logger,
}

impl SecretLoader {
    #[allow(missing_docs)]
    pub fn new(secret_path: String, log: Logger) -> Self {
        SecretLoader { secret_path, log }
    }

    /// Gets the secret.
    pub fn load(self) -> InternalResult<String> {
        self.get_secret_from_file()
    }

    /// Gets the secret from a file.
    fn get_secret_from_file(&self) -> InternalResult<String> {
        info!(self.log, "Reading secret from file"; "path" => &self.secret_path);

        let mut secret_file = to_internal_result(
            fs::File::open(&self.secret_path)
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

    // TODO: Support reading secret from CLI.
}
