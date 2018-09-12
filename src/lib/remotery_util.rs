#![macro_use]
//! Utility functions and macros for using the
//! [Remotery](https://github.com/Celtoys/Remotery) profiling tool

#[cfg(feature = "use-remotery")]
use remotery;
use slog::Logger;

/// Initialize remotery
#[cfg(feature = "use-remotery")]
pub fn initialize_remotery(log: &Logger) -> remotery::Remotery {
    info!(log, "Initializing remotery");
    remotery::Remotery::create_global_instance()
        .expect("Failed to initialize remotery")
}

#[allow(missing_docs)]
#[cfg(not(feature = "use-remotery"))]
pub fn initialize_remotery(_log: &Logger) -> () {}

#[cfg(feature = "use-remotery")]
macro_rules! remotery_scope {
    ($scope_name:expr) => {
        let _remotery_scope = ::remotery::RemoteryScope::new(
            $scope_name,
            ::remotery::SampleFlags::Default,
        );
    };
}

#[cfg(not(feature = "use-remotery"))]
macro_rules! remotery_scope {
    ($scope_name:expr) => {};
}
