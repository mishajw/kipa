error_chain! {
    errors {
        ParseError(s: String) { display("Parse error: {}", s) }
        JoinError(s: String) { display("Join error: {}", s) }
        ConfigError(s: String) { display("Configuration error: {}", s) }
    }
}

