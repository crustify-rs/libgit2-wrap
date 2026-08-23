//! Safe wrappers for libgit2 config APIs.

use ffibox::CBox;

use crate::ffi;

/// Wraps: git_config_level_t
/// Priority level of a libgit2 configuration source.
///
/// Values at or above [`Self::APP`] are reserved for application-defined
/// configuration sources. [`Self::HIGHEST`] is the query sentinel that asks
/// libgit2 to select the highest level currently loaded.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GitConfigLevel(ffi::git_config_level_t);

impl GitConfigLevel {
    /// The system-wide configuration file.
    pub const SYSTEM: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_SYSTEM);
    /// The XDG configuration file.
    pub const XDG: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_XDG);
    /// The user-specific global configuration file.
    pub const GLOBAL: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_GLOBAL);
    /// The repository-local configuration file.
    pub const LOCAL: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_LOCAL);
    /// The worktree-specific configuration file.
    pub const WORKTREE: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_WORKTREE);
    /// The first level available to application-defined configuration sources.
    pub const APP: Self = Self(ffi::git_config_level_t_GIT_CONFIG_LEVEL_APP);
    /// Sentinel selecting the highest configuration level currently loaded.
    pub const HIGHEST: Self = Self(ffi::git_config_level_t_GIT_CONFIG_HIGHEST_LEVEL);

    /// Converts a C value when it denotes a published or application level.
    pub const fn from_raw(raw: ffi::git_config_level_t) -> Option<Self> {
        if raw == Self::HIGHEST.0
            || (raw >= Self::SYSTEM.0 && raw <= Self::WORKTREE.0)
            || raw >= Self::APP.0
        {
            Some(Self(raw))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 configuration level.
    pub const fn as_raw(self) -> ffi::git_config_level_t {
        self.0
    }

    /// Returns whether this is an application-defined level.
    pub const fn is_application(self) -> bool {
        self.0 >= Self::APP.0
    }
}

ffibox::define_ctype!(
    /// Wraps: git_config
    /// Opaque, refcounted configuration object managed by libgit2.
    GitConfig,
    GitConfigRef,
    GitConfigMut,
    ffi::git_config
);

/// An owned reference count to a [`GitConfig`].
///
/// Dropping it calls `git_config_free`, which releases one count. Libgit2 does
/// not publish an operation that increments a config's reference count, so
/// this owner intentionally does not implement `Clone`.
pub type GitConfigOwned = CBox<GitConfig>;

// SAFETY: `git_config_free` consumes exactly one reference to a fully formed
// `git_config`; it decrements the embedded count and releases the allocation
// and its backend fields when that was the final reference. It accepts null,
// although `CBox` always supplies a non-null pointer.
ffibox::impl_dropped!(GitConfig, ffi::git_config, ffi::git_config_free);

/// Wraps: git_config_parse_bool
/// Parses a libgit2 boolean spelling. `None` produces libgit2's parse error.
pub fn git_config_parse_bool(value: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_bool(&mut out, value) };
    if status == 0 {
        Ok(out != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_config_parse_int32
/// Parses a 32-bit integer, including libgit2's `k`, `m`, and `g` suffixes.
pub fn git_config_parse_int32(value: Option<&core::ffi::CStr>) -> Result<i32, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_int32(&mut out, value) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_parse_int64
/// Parses a 64-bit integer, including libgit2's `k`, `m`, and `g` suffixes.
pub fn git_config_parse_int64(value: Option<&core::ffi::CStr>) -> Result<i64, i32> {
    let mut out = 0;
    let value = value.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `out` is writable and `value` is null or a live C string.
    let status = unsafe { ffi::git_config_parse_int64(&mut out, value) };
    if status == 0 { Ok(out) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use core::ptr;

    use super::*;

    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard. The config owner is dropped before the guard.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn published_levels_round_trip() {
        let levels = [
            GitConfigLevel::SYSTEM,
            GitConfigLevel::XDG,
            GitConfigLevel::GLOBAL,
            GitConfigLevel::LOCAL,
            GitConfigLevel::WORKTREE,
            GitConfigLevel::APP,
            GitConfigLevel::HIGHEST,
        ];

        for level in levels {
            assert_eq!(GitConfigLevel::from_raw(level.as_raw()), Some(level));
        }
    }

    #[test]
    fn application_levels_are_extensible_but_gaps_are_rejected() {
        let custom = GitConfigLevel::from_raw(GitConfigLevel::APP.as_raw() + 17)
            .expect("application levels begin at APP");
        assert!(custom.is_application());

        assert_eq!(GitConfigLevel::from_raw(0), None);
        assert_eq!(GitConfigLevel::from_raw(1), None);
        assert_eq!(GitConfigLevel::from_raw(-2), None);
    }

    #[test]
    fn public_scalar_parsers_return_typed_values() {
        assert_eq!(git_config_parse_bool(Some(c"yes")), Ok(true));
        assert_eq!(git_config_parse_bool(Some(c"off")), Ok(false));
        assert_eq!(git_config_parse_int32(Some(c"2k")), Ok(2048));
        assert_eq!(git_config_parse_int64(Some(c"3m")), Ok(3 * 1024 * 1024));
    }

    #[test]
    fn config_owner_provides_shared_and_exclusive_handles() {
        let _init = Libgit2Init::acquire();
        let mut raw = ptr::null_mut();

        // SAFETY: libgit2 is initialized and `raw` is a writable out slot.
        assert_eq!(unsafe { ffi::git_config_new(&mut raw) }, 0);
        // SAFETY: a successful `git_config_new` returns a fresh owned count,
        // whose matching down-reference operation is `git_config_free`.
        let mut config = unsafe { GitConfigOwned::from_raw(raw) }
            .expect("git_config_new returned success with a null config");

        let shared = config.as_ref();
        assert_eq!(shared.as_ptr(), raw.cast_const());

        let mut exclusive = config.as_mut();
        assert_eq!(exclusive.as_mut_ptr(), raw);
        assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
    }
}
