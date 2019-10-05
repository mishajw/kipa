//! Error types defined by the API.

use std::fmt;

/// Error returned when a request has failed.
#[derive(Clone, Debug)]
pub struct ApiError {
    /// Description of the error.
    pub message: String,
    /// Type of the error.
    pub error_type: ApiErrorType,
}

impl ApiError {
    #[allow(missing_docs)]
    pub fn new(message: String, error_type: ApiErrorType) -> Self {
        ApiError {
            message,
            error_type,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ApiError({})", self.message)
    }
}

/// Errors returned by the API.
#[derive(Clone, Debug)]
pub enum ApiErrorType {
    /// No error occurred.
    // TODO: Can we remove this enum?
    NoError = 0,
    /// Error in parsing user input.
    Parse = 1,
    /// Error in configuration, e.g. in the daemon or CLI.
    Configuration = 2,
    /// Error caused by an external library/tool.
    External = 3,
    /// Misc. errors that are not exposed to user.
    Internal = 4,
}

impl fmt::Display for ApiErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            ApiErrorType::NoError => "No error",
            ApiErrorType::Parse => "Parse error",
            ApiErrorType::Configuration => "Configuration error",
            ApiErrorType::External => "External error",
            ApiErrorType::Internal => "Internal error",
        };
        write!(f, "{:02} {}", self.clone() as u8, name)
    }
}

/// Result for `ApiError`s.
pub type ApiResult<T> = Result<T, ApiError>;
