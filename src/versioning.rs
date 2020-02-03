use error::*;

use failure::_core::cmp::Ordering;
use regex::Regex;

/// Versions >= to this version are compatible.
const FIRST_STABLE_VERSION: Version = Version {
    major: 0,
    minor: 2,
    patch: 3,
};

/// Get the version of this binary.
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Verify that two versions are compatible.
pub fn verify_version(our_version_str: &str, their_version_str: &str) -> ApiResult<()> {
    let our_version = Version::from_str(our_version_str)?;
    let their_version = Version::from_str(their_version_str)?;
    if (&our_version >= &FIRST_STABLE_VERSION) != (&their_version >= &FIRST_STABLE_VERSION) {
        return Err(ApiError::new(
            format!(
                "One version is below first stable version: {}, {}",
                our_version_str, their_version_str
            ),
            ApiErrorType::Parse,
        ));
    }
    // More logic can be added when we change the API.
    Ok(())
}

/// A semver version.
#[derive(PartialEq, Eq)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    fn from_str(version_str: &str) -> ApiResult<Version> {
        lazy_static! {
            static ref VERSION_REGEX: Regex =
                Regex::new(r"(?P<maj>\d+)\.(?P<min>\d+)\.(?P<patch>\d+)",)
                    .expect("Failed to compile version regex");
        }
        let parsed = VERSION_REGEX.captures(version_str).ok_or_else(|| {
            ApiError::new(
                "Error on parsing our version number".into(),
                ApiErrorType::Parse,
            )
        })?;
        Ok(Version {
            major: parsed["maj"].parse().unwrap(),
            minor: parsed["min"].parse().unwrap(),
            patch: parsed["patch"].parse().unwrap(),
        })
    }
}

impl PartialOrd for &Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for &Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spectral::assert_that;
    use spectral::result::*;

    #[test]
    fn test_verify_version() {
        // Before stable version is allowed.
        assert_that!(verify_version("0.0.0", "0.2.1")).is_ok();
        // After stable version is allowed.
        assert_that!(verify_version("2.1.2", "1.1.2")).is_ok();
        // Mix is not allowed.
        assert_that!(verify_version("0.0.0", "0.3.0")).is_err();
    }
}
