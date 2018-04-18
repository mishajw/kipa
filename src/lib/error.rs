//! Error types used in KIPA lib.

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
    }
}
