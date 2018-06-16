//! Error types used across project
//!
//! There are four main error/result types used across the project:
//! 1. `kipa_lib::error::{Error, Result}`: Error types created by `error_chain`,
//!    used for code that does not directly interact with external agents (e.g.
//!    other nodes, CLI).
//! 2. `kipa_lib::api::{ApiError, ApiResult}`: Error types that are
//!    public-facing (e.g. seen by other nodes and the CLI).
//! 3. `kipa_lib::error::{InternalError, InternalResult}`: Error types that can
//!    either represent an internal error (`PrivateError`) or a public-facing
//!    error (`PublicError`). These types should be used to propagate errors
//!    from functionality that can produce public-facing errors, but can also
//!    have internal errors that should not be public-facing. This should be
//!    typically used across all request handling until the highest level, when
//!    it is converted to an `{ApiError, ApiResult}` in order to be sent
//!    publicly.
//! 4. `kipa_lib::error::{ResponseError, ResponseResult}`: Error types that
//!    represent a response from a different node, with two options for errors:
//!    either a public error that has been received from the other node, or an
//!    error that occurred when receiving this response.

use api::{ApiError, ApiResult};

use std::fmt;

/// Errors generated using `error_chain` module
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
    PublicError(ApiError),
    /// Private error wrapping `Error` (from `error_chain`)
    PrivateError(ErrorKind),
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::PublicError(err) => {
                write!(f, "Public error: {}", err)
            }
            InternalError::PrivateError(err) => {
                write!(f, "Private error: {}", err)
            }
        }
    }
}

/// Result type with `InternalError` as the error type, should not be seen
/// externally
pub type InternalResult<T> = ::std::result::Result<T, InternalError>;

/// Representation of an error that can be caused when getting a response from
/// another node
///
/// Programmatically identical to `InternalError`, but semantically different.
pub type ResponseError = InternalError;

/// Result type with `ResponseError` as error type
pub type ResponseResult<T> = InternalResult<T>;

// TODO: Change all conversion types into `Into` trait impls, waiting on
// rfc/1023

/// Convert a result into an internal result
pub fn to_internal_result<T>(result: Result<T>) -> InternalResult<T> {
    result.map_err(|err| InternalError::PrivateError(err.into()))
}

/// Convert an API result into an internal result
pub fn api_to_internal_result<T>(result: ApiResult<T>) -> InternalResult<T> {
    result.map_err(|err| InternalError::PublicError(err.into()))
}

/// Turn an internal result into a public-facing `ApiResult`
pub fn to_api_result<T>(result: InternalResult<T>) -> ApiResult<T> {
    // TODO: Implement
    result.map_err(|err| match err {
        InternalError::PublicError(err) => err,
        // TODO: How to log the lost error?
        InternalError::PrivateError(_) => {
            ApiError::new("Internal error".into())
        }
    })
}
