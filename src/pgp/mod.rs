//! Loading and using OpenPGP keys.

mod gnupg_key_loader;
mod pgp_key_handler;
mod secret_loader;

/// Default path for secret file.
pub const DEFAULT_SECRET_PATH: &str = "./secret.txt";

pub use self::gnupg_key_loader::GnupgKeyLoader;
pub use self::pgp_key_handler::PgpKeyHandler;
pub use self::secret_loader::SecretLoader;
