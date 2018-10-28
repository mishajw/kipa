# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic
Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Add
- Add use of encryption and signatures to messaging between daemons
- Add verification message to ensure that an IP address belongs to a certain key.
  This message is used to verify the result of a key search
- Add garbage collection of neighbours, i.e. removing neighbours once they stop
  returning valid responses to validation requests
- Add use of verification message when adding a new neighbour, in order to
  verify their address
- Add remotery profiling
- Add log verbose levels
- Add customizable log location

### Refactor
- Move `src/lib/socket_server/mod.rs` to `src/lib/server/socket_server.rs`
- Move usage of `DataTransformer` from servers into `MessageHandler`
- Rename "benchmarks" to "simulation"

## 0.1.2 - 2018-06-24

### Added
- Complete KIPA implementation, with the following notable exceptions:
  - No security (authenticity, secrecy, etc.)
  - Can not deal with changes in IP address over time
- Benchmarks with end to end tests
- Documentation (implementation comments and write-up)

[Unreleased]: https://github.com/mishajw/kipa/compare/v0.1.2...HEAD

