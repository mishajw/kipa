//! Error types defined by the API

use std::fmt;

/// Possible API errors
#[derive(Clone, Debug)]
pub enum ApiErrorType {
    /// No error occurred
    NoError = 0,
    /// Error in parsing user input
    Parse = 1,
    /// Error in configuration of daemon/CLI
    Configuration = 2,
    /// Error caused by an external library/tool
    External = 3,
    /// Misc errors that are not exposed to user
    Internal = 4,
}

/// Error returned when a request has failed
#[derive(Clone, Debug)] // Derive `Debug` to return from main function
pub struct ApiError {
    /// Description of the error
    pub message: String,
    /// Type of the error
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

/// Result for `ApiError`s
pub type ApiResult<T> = Result<T, ApiError>;
