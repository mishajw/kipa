//! Error types used across KIPA.
//!
//! There are four main error/result types used:
//! 1. `kipa_lib::api::{ApiError, ApiResult}`: Public errors that are seen by other nodes and the
//!    CLI.
//! 2. `kipa_lib::error::{Error, Result}`: Private errors created by `error_chain`, typically seen
//!    only in logs.
//! 3. `kipa_lib::error::{InternalError, InternalResult}`: Either a public or private error (i.e.
//!    `ApiError` or `Error`). Used in functions where we potentially return API errors, but can
//!    also fail internally. These can be converted to `ApiError`s, where in the case that it is a
//!    private error, the `ApiErrorType::InternalError` is used.

pub use api::error::{ApiError, ApiErrorType, ApiResult};

use error_chain;
use error_chain::ChainedError;
use failure;
use slog::Logger;
use std::fmt;

error_chain! {
    errors {
        /// Error in parsing some data
        ParseError(s: String) { display("Parse error: {}", s) }
        /// Error in joining on a thread
        JoinError(s: String) { display("Join error: {}", s) }
        /// Error in configuration set up
        ConfigError(s: String) { display("Configuration error: {}", s) }
        /// Error in the request
        RequestError(s: String) { display("Request error: {}", s) }
        /// Error in the response type
        ResponseError(s: String) { display("Response error: {}", s) }
        /// Error in retrieving IP address
        IpAddressError(s: String) { display("IP address error: {}", s) }
        /// Error due to unimplemented functionality
        UnimplementedError(s: String) { display("Unimplemented error: {}", s) }
        /// Error due to GPGME
        GpgError(s: String, error: failure::Error) {
            display("GPG error: {}. Caused by: {}", s, error)
        }
    }
}

/// Representation of an error for internal code
///
/// This error type must not be seen externally, e.g. not reflected to other
/// nodes or to the CLI.
///
/// If it needs to be, must be converted into an `ApiError` using
/// `to_api_error()`.
pub enum InternalError {
    /// Public error wrapping `ApiError`
    PublicError(ApiError, Option<Error>),
    /// Private error wrapping `Error` (from `error_chain`)
    PrivateError(Error),
}

impl InternalError {
    /// Helper function to create a public error
    pub fn public(s: &str, error_type: ApiErrorType) -> InternalError {
        InternalError::PublicError(ApiError::new(s.into(), error_type), None)
    }

    /// Helper function to create a public error that was caused by a private
    /// error
    pub fn public_with_error<E>(s: &str, error_type: ApiErrorType, error: E) -> InternalError
    where
        E: ::std::error::Error + Send + 'static,
    {
        // Copied and editted from `error_chain`'s `chain_err` function
        let state = error_chain::State::new::<Error>(Box::new(error));
        let new_error = error_chain::ChainedError::new(s.into(), state);
        InternalError::PublicError(ApiError::new(s.into(), error_type), Some(new_error))
    }

    /// Helper function to create a private error
    pub fn private<E: Into<Error>>(err: E) -> InternalError {
        InternalError::PrivateError(err.into())
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::PublicError(pub_err, Some(priv_err)) => {
                write!(f, "Public error: {}, caused by {}", pub_err, priv_err)
            }
            InternalError::PublicError(pub_err, None) => write!(f, "Public error: {}", pub_err),
            InternalError::PrivateError(err) => write!(f, "Private error: {}", err.display_chain()),
        }
    }
}

/// Result type with `InternalError` as the error type, should not be seen
/// externally
pub type InternalResult<T> = ::std::result::Result<T, InternalError>;

// TODO: Change all conversion types into `Into` trait impls, waiting on rfc/1023

/// Convert a result into an internal result
pub fn to_internal_result<T>(result: Result<T>) -> InternalResult<T> {
    result.map_err(|err| InternalError::PrivateError(err.into()))
}

/// Convert an API result into an internal result
pub fn api_to_internal_result<T>(result: ApiResult<T>) -> InternalResult<T> {
    result.map_err(|err| InternalError::PublicError(err, None))
}

/// Turn an internal result into a public-facing `ApiResult`
pub fn to_api_result<T>(result: InternalResult<T>, log: &Logger) -> ApiResult<T> {
    result.map_err(|err| match err {
        InternalError::PublicError(err, _) => err,
        InternalError::PrivateError(err) => {
            info!(
                log, "Private error when casting to API error";
                "err" => %err.display_chain());
            ApiError::new("Internal error".into(), ApiErrorType::Internal)
        }
    })
}
