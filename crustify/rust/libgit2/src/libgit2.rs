//! Safe wrappers for libgit2 libgit2 APIs.

use crate::ffi;

/// Wraps: git_libgit2_features
/// Compile-time features present in the linked libgit2.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Libgit2Features(i32);

impl Libgit2Features {
    pub const THREADS: Self = Self(ffi::git_feature_t_GIT_FEATURE_THREADS as i32);
    pub const HTTPS: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTPS as i32);
    pub const SSH: Self = Self(ffi::git_feature_t_GIT_FEATURE_SSH as i32);
    pub const NSEC: Self = Self(ffi::git_feature_t_GIT_FEATURE_NSEC as i32);
    pub const HTTP_PARSER: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP_PARSER as i32);
    pub const REGEX: Self = Self(ffi::git_feature_t_GIT_FEATURE_REGEX as i32);
    pub const I18N: Self = Self(ffi::git_feature_t_GIT_FEATURE_I18N as i32);
    pub const AUTH_NTLM: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NTLM as i32);
    pub const AUTH_NEGOTIATE: Self = Self(ffi::git_feature_t_GIT_FEATURE_AUTH_NEGOTIATE as i32);
    pub const COMPRESSION: Self = Self(ffi::git_feature_t_GIT_FEATURE_COMPRESSION as i32);
    pub const SHA1: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA1 as i32);
    pub const SHA256: Self = Self(ffi::git_feature_t_GIT_FEATURE_SHA256 as i32);
    pub const HTTP: Self = Self(ffi::git_feature_t_GIT_FEATURE_HTTP as i32);

    #[must_use]
    pub const fn bits(self) -> i32 {
        self.0
    }

    #[must_use]
    pub const fn contains(self, feature: Self) -> bool {
        self.0 & feature.0 == feature.0
    }
}

/// Returns the linked library's compile-time feature bit set.
#[must_use]
pub fn git_libgit2_features() -> Libgit2Features {
    // SAFETY: the query has no arguments and returns a scalar bit set.
    Libgit2Features(unsafe { ffi::git_libgit2_features() })
}

/// Wraps: git_libgit2_version
/// Runtime version of the linked libgit2 library.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Libgit2Version {
    pub major: i32,
    pub minor: i32,
    pub revision: i32,
}

/// Queries the runtime version of the linked libgit2 library.
pub fn git_libgit2_version() -> Result<Libgit2Version, i32> {
    let (mut major, mut minor, mut revision) = (0, 0, 0);
    // SAFETY: all three arguments are distinct writable scalar out-slots.
    let status = unsafe { ffi::git_libgit2_version(&mut major, &mut minor, &mut revision) };
    if status == 0 {
        Ok(Libgit2Version {
            major,
            minor,
            revision,
        })
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_version_is_queryable() {
        let version = git_libgit2_version().unwrap();
        assert!(version.major >= 0 && version.minor >= 0 && version.revision >= 0);
    }

    #[test]
    fn mandatory_features_are_reported() {
        let features = git_libgit2_features();
        assert!(features.contains(Libgit2Features::HTTP_PARSER));
        assert!(features.contains(Libgit2Features::REGEX));
    }
}
