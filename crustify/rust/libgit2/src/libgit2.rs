//! Safe wrappers for libgit2 libgit2 APIs.

use crate::ffi;

/// One owned count in libgit2's process-global initialization state.
///
/// Pass the token to [`git_libgit2_shutdown`] after every libgit2 object and
/// borrow that may depend on this count is gone. Dropping the token instead
/// deliberately leaks the count, which keeps the global state initialized.
#[derive(Debug)]
#[must_use = "dropping this token leaves its libgit2 initialization count active"]
pub struct Libgit2Init {
    initial_count: usize,
}

impl Libgit2Init {
    /// Returns the initialization count reported when this token was acquired.
    #[must_use]
    pub const fn initial_count(&self) -> usize {
        self.initial_count
    }
}

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

/// Wraps: git_libgit2_features
/// Returns the linked library's compile-time feature bit set.
#[must_use]
pub fn git_libgit2_features() -> Libgit2Features {
    // SAFETY: the query has no arguments and returns a scalar bit set.
    Libgit2Features(unsafe { ffi::git_libgit2_features() })
}

/// Runtime version of the linked libgit2 library.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Libgit2Version {
    pub major: i32,
    pub minor: i32,
    pub revision: i32,
}

/// Wraps: git_libgit2_version
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

/// Wraps: git_libgit2_init
/// Acquires one process-global libgit2 initialization count.
pub fn git_libgit2_init() -> Result<Libgit2Init, i32> {
    // SAFETY: initialization takes no arguments, is internally synchronized,
    // and a successful count is balanced by the returned owning token.
    let count = unsafe { ffi::git_libgit2_init() };
    if count <= 0 {
        Err(count)
    } else {
        Ok(Libgit2Init {
            initial_count: usize::try_from(count).expect("a positive C int fits usize"),
        })
    }
}

/// Wraps: git_libgit2_prerelease
/// Returns the linked library's static prerelease label, if this is not a
/// final release.
#[must_use]
pub fn git_libgit2_prerelease() -> Option<&'static core::ffi::CStr> {
    // SAFETY: libgit2 returns null or a pointer to an immutable compile-time
    // NUL-terminated string that remains valid for the process lifetime.
    let prerelease = unsafe { ffi::git_libgit2_prerelease() };
    if prerelease.is_null() {
        None
    } else {
        // SAFETY: the non-null result has the static C-string contract above.
        Some(unsafe { core::ffi::CStr::from_ptr(prerelease) })
    }
}

/// Wraps: git_libgit2_shutdown
/// Consumes one initialization token and returns the number of counts left.
///
/// # Safety
///
/// If this may release the final process-global count, every libgit2 object,
/// borrowed result and concurrent operation that relies on initialized global
/// state must already be gone. That global relationship cannot be expressed by
/// the token's type.
pub unsafe fn git_libgit2_shutdown(_initialization: Libgit2Init) -> Result<usize, i32> {
    // SAFETY: the caller supplies the global quiescence obligation, while the
    // consumed token proves this call owns one unmatched successful init.
    let remaining = unsafe { ffi::git_libgit2_shutdown() };
    if remaining < 0 {
        Err(remaining)
    } else {
        Ok(usize::try_from(remaining).expect("a nonnegative C int fits usize"))
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

    #[test]
    fn initialization_token_balances_explicit_shutdown() {
        let initialization = git_libgit2_init().unwrap();
        assert!(initialization.initial_count() > 0);
        // SAFETY: this test has created no libgit2 object or borrowed result,
        // so its initialization count can be released immediately.
        unsafe { git_libgit2_shutdown(initialization) }.unwrap();
    }

    #[test]
    fn release_prerelease_label_is_optional_static_data() {
        assert_eq!(git_libgit2_prerelease(), None);
    }
}
