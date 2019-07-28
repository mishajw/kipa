use error::*;

use regex::Regex;

/// Get the version of this binary
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Verify that two versions are compatible
pub fn verify_version(our_version: &str, their_version: &str) -> ApiResult<()> {
    lazy_static! {
        static ref VERSION_REGEX: Regex =
            Regex::new(r"(?P<maj>\d+)\.(?P<min>\d+)\.(?P<patch>\d+)",)
                .expect("Failed to compile version regex");
    }

    let ours_parsed =
        VERSION_REGEX.captures(our_version).ok_or(ApiError::new(
            "Error on parsing our version number".into(),
            ApiErrorType::Parse,
        ))?;
    let theirs_parsed =
        VERSION_REGEX.captures(their_version).ok_or(ApiError::new(
            "Error on parsing our version number".into(),
            ApiErrorType::Parse,
        ))?;

    if ours_parsed["maj"] != theirs_parsed["maj"] {
        return Err(ApiError::new(
            format!(
                "Major versions do not match: {} != {}",
                our_version, their_version
            ),
            ApiErrorType::Parse,
        ));
    }

    if &ours_parsed["maj"] == "0"
        && (&ours_parsed["min"] != &theirs_parsed["min"]
            || &ours_parsed["patch"] != &theirs_parsed["patch"])
    {
        return Err(ApiError::new(
            format!(
                "In beta (<1.0.0) minor and patch numbers need to match: {} \
                 != {}",
                our_version, their_version
            ),
            ApiErrorType::Parse,
        ));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use spectral::assert_that;
    use spectral::result::*;

    #[test]
    fn test_verify_version() {
        // Identical versions should succeed
        assert_that!(verify_version("0.0.0", "0.0.0")).is_ok();
        assert_that!(verify_version("2.0.0", "2.0.0")).is_ok();
        assert_that!(verify_version("2.1.2", "2.1.2")).is_ok();

        // Non-identical beta (`0.*`) versions should fail
        assert_that!(verify_version("0.0.0", "0.0.1")).is_err();

        // Identical major versions should succeed
        assert_that!(verify_version("1.0.0", "1.0.1")).is_ok();
        assert_that!(verify_version("2.1.0", "2.0.0")).is_ok();
        assert_that!(verify_version("3.1.0", "3.0.1")).is_ok();

        // Non-identical major versions should fail
        assert_that!(verify_version("1.0.0", "2.0.0")).is_err();
        assert_that!(verify_version("10.2.1", "33.2.1")).is_err();
    }
}
