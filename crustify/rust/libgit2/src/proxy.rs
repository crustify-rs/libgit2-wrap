//! Safe wrappers for libgit2 proxy APIs.

use crate::ffi;

/// Wraps: git_proxy_t
/// Selects how libgit2 discovers or connects to a proxy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum ProxyType {
    /// Do not explicitly connect through a proxy.
    #[default]
    None = ffi::git_proxy_t_GIT_PROXY_NONE,
    /// Discover the proxy from Git configuration.
    Auto = ffi::git_proxy_t_GIT_PROXY_AUTO,
    /// Connect through a caller-specified proxy URL.
    Specified = ffi::git_proxy_t_GIT_PROXY_SPECIFIED,
}

/// An integer that is not a published [`ProxyType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidProxyType(ffi::git_proxy_t);

impl InvalidProxyType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_proxy_t {
        self.0
    }
}

impl From<ProxyType> for ffi::git_proxy_t {
    fn from(proxy_type: ProxyType) -> Self {
        proxy_type as Self
    }
}

impl TryFrom<ffi::git_proxy_t> for ProxyType {
    type Error = InvalidProxyType;

    fn try_from(proxy_type: ffi::git_proxy_t) -> Result<Self, Self::Error> {
        match proxy_type {
            ffi::git_proxy_t_GIT_PROXY_NONE => Ok(Self::None),
            ffi::git_proxy_t_GIT_PROXY_AUTO => Ok(Self::Auto),
            ffi::git_proxy_t_GIT_PROXY_SPECIFIED => Ok(Self::Specified),
            value => Err(InvalidProxyType(value)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn proxy_types_round_trip_through_the_c_type() {
        for proxy_type in [ProxyType::None, ProxyType::Auto, ProxyType::Specified] {
            let raw = ffi::git_proxy_t::from(proxy_type);
            assert_eq!(ProxyType::try_from(raw), Ok(proxy_type));
        }
    }

    #[test]
    fn invalid_proxy_type_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_proxy_t_GIT_PROXY_SPECIFIED + 1;
        let error = ProxyType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn proxy_type_matches_the_c_abi_scalar() {
        assert_eq!(size_of::<ProxyType>(), size_of::<ffi::git_proxy_t>());
        assert_eq!(align_of::<ProxyType>(), align_of::<ffi::git_proxy_t>());
    }

    #[test]
    fn public_initializer_returns_owned_proxy_options() {
        let options = git_proxy_options_init(ffi::GIT_PROXY_OPTIONS_VERSION)
            .expect("the published proxy-options version initializes");
        assert_eq!(options.as_ref().version(), ffi::GIT_PROXY_OPTIONS_VERSION);
        assert_eq!(options.as_ref().proxy_type(), Ok(ProxyType::None));
    }

    #[test]
    fn an_unsupported_version_yields_no_options_at_all() {
        // The rejection path reports through `git_error_set`, which needs the
        // thread state libgit2 installs at initialization.
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        // `git_error__check_version` accepts `0 < version <= current` only, so
        // both ends are refused and the wrapper hands back no options rather
        // than a partially written record.
        assert_eq!(git_proxy_options_init(0).err(), Some(-1));
        assert_eq!(
            git_proxy_options_init(ffi::GIT_PROXY_OPTIONS_VERSION + 1).err(),
            Some(-1)
        );
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_proxy_options_init
/// Creates proxy options initialized for `version`.
pub fn git_proxy_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::proxy::GitProxyOptions<'data>>, i32> {
    let mut options = crate::api::proxy::GitProxyOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and the C
    // initializer retains no pointer to it or to any of its cleared fields.
    let status = unsafe { ffi::git_proxy_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}
