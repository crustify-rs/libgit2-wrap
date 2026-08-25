//! Safe wrappers for libgit2 config APIs.

use core::ffi::{CStr, c_void};
use core::ptr::addr_of;
use ffibox::CBox;

use crate::ffi;

unsafe extern "C" fn config_foreach_trampoline<C: crate::api::config::GitConfigForeachCallback>(
    entry: *const ffi::git_config_entry,
    payload: *mut c_void,
) -> i32 {
    if entry.is_null() || payload.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: each wrapper pairs this monomorphized trampoline with the
    // address of its exclusively borrowed callback for a synchronous walk.
    let callback = unsafe { &mut *payload.cast::<C>() };
    // SAFETY: libgit2 supplies a live transient entry for this callback
    // invocation and the null case was rejected above.
    let entry = unsafe { GitConfigEntryRef::from_ptr(entry.cast_mut()) }
        .expect("the callback rejected null");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback.call(entry)))
        .unwrap_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_config_iterator_free
/// An owned configuration iterator that keeps its source configuration borrowed.
///
/// The coupling is a stored pointer, not a naming convention:
/// `git_config_iterator_new` and `git_config_iterator_glob_new` assign
/// `iter->config = config` in `src/libgit2/config.c` without taking a
/// reference count, and `all_iter_next` reads `iter->config->readers` again on
/// every advance to reach the next backend. The multivar iterator wraps one of
/// those, so it inherits the same borrow. Outliving the configuration is
/// therefore rejected:
///
/// ```compile_fail
/// use libgit2::config::{git_config_iterator_new, git_config_new, GitConfigIteratorOwned};
///
/// fn escape() -> GitConfigIteratorOwned<'static> {
///     let config = git_config_new().unwrap();
///     git_config_iterator_new(config.as_ref()).unwrap()
/// }
/// ```
pub struct GitConfigIteratorOwned<'config> {
    inner: crate::sys::config::GitConfigIteratorOwned,
    _config: core::marker::PhantomData<GitConfigRef<'config>>,
}

impl GitConfigIteratorOwned<'_> {
    /// Borrows the iterator without permitting it to advance.
    #[must_use]
    pub fn as_ref(&self) -> crate::sys::config::GitConfigIteratorRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the iterator exclusively for advancing.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::sys::config::GitConfigIteratorMut<'_> {
        self.inner.as_mut()
    }
}

fn iterator_result<'config>(
    status: i32,
    inner: Option<crate::sys::config::GitConfigIteratorOwned>,
) -> Result<GitConfigIteratorOwned<'config>, i32> {
    if status == 0 {
        Ok(GitConfigIteratorOwned {
            inner: inner.ok_or(ffi::git_error_code_GIT_ERROR)?,
            _config: core::marker::PhantomData,
        })
    } else {
        drop(inner);
        Err(status)
    }
}

#[cfg(test)]
mod iterator_owner_result_tests {
    use super::*;

    #[test]
    fn typed_iterator_result_rejects_a_null_success_output() {
        assert!(matches!(
            iterator_result::<'static>(0, None),
            Err(ffi::git_error_code_GIT_ERROR)
        ));
    }

    #[test]
    fn typed_iterator_result_preserves_a_constructor_error() {
        assert!(matches!(iterator_result::<'static>(-7, None), Err(-7)));
    }
}

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
/// Dropping it calls `git_config_free`, which releases one count. No exported
/// operation takes a `git_config *` and yields an additional count: the
/// refcount is incremented only where libgit2 hands out a config it already
/// holds, such as `git_repository_config`. This owner therefore intentionally
/// does not implement `Clone`.
pub type GitConfigOwned = CBox<GitConfig>;

// SAFETY: `git_config_free` consumes exactly one reference to a fully formed
// `git_config`; it decrements the embedded count and releases the allocation
// and its backend fields when that was the final reference. It accepts null,
// although `CBox` always supplies a non-null pointer.
ffibox::impl_dropped!(GitConfig, ffi::git_config, ffi::git_config_free);

/// Wraps: git_config_parse_bool
/// Parses a libgit2 boolean spelling.
///
/// `None` is not an error: `git__parse_bool` treats a missing value as the
/// bare `[section] key` form and yields `true`. An empty string yields
/// `false`, and anything else falls through to the integer parser.
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
    use core::mem::{align_of, size_of};

    use ffibox::CDropped;

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
    fn config_representation_matches_the_c_seam_and_registers_drop() {
        fn assert_dropped<T: CDropped>() {}

        assert_eq!(size_of::<GitConfig>(), size_of::<ffi::git_config>());
        assert_eq!(align_of::<GitConfig>(), align_of::<ffi::git_config>());
        assert_eq!(
            size_of::<GitConfigRef<'_>>(),
            size_of::<*const ffi::git_config>()
        );
        assert_eq!(
            size_of::<GitConfigMut<'_>>(),
            size_of::<*mut ffi::git_config>()
        );
        assert_eq!(
            size_of::<GitConfigOwned>(),
            size_of::<*mut ffi::git_config>()
        );
        assert_dropped::<GitConfig>();
    }

    #[test]
    fn public_scalar_parsers_return_typed_values() {
        // Libgit2 requires initialization before any entry point; these
        // parsers reach `git_error_set` on their failure paths.
        let _init = Libgit2Init::acquire();
        assert_eq!(git_config_parse_bool(Some(c"yes")), Ok(true));
        assert_eq!(git_config_parse_bool(Some(c"off")), Ok(false));
        assert_eq!(git_config_parse_int32(Some(c"2k")), Ok(2048));
        assert_eq!(git_config_parse_int64(Some(c"3m")), Ok(3 * 1024 * 1024));
    }

    #[test]
    fn absent_values_follow_each_parser_rather_than_the_pointer() {
        // These parsers reach `git_error_set` on their failure paths, which
        // requires an initialized libgit2 just as the successful ones do.
        let _init = Libgit2Init::acquire();
        // A valueless key is `true` for the boolean parser, ...
        assert_eq!(git_config_parse_bool(None), Ok(true));
        assert_eq!(git_config_parse_bool(Some(c"")), Ok(false));
        // ... but the integer parsers reject a missing string outright.
        assert!(git_config_parse_int32(None).is_err());
        assert!(git_config_parse_int64(None).is_err());
    }

    #[test]
    fn config_owner_provides_shared_and_exclusive_handles() {
        let _init = Libgit2Init::acquire();
        let mut config = git_config_new().expect("an empty config can be allocated");
        let raw = config.as_ptr();

        let shared = config.as_ref();
        assert_eq!(shared.as_ptr(), raw.cast_const());

        let mut exclusive = config.as_mut();
        assert_eq!(exclusive.as_mut_ptr(), raw);
        assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
    }

    #[test]
    fn config_error_result_releases_a_typed_owner() {
        let _init = Libgit2Init::acquire();
        let config = git_config_new().expect("an empty config can be allocated");

        assert!(matches!(config_result(-123, Some(config)), Err(-123)));
    }

    #[test]
    fn config_snapshot_returns_an_independent_owner() {
        let _init = Libgit2Init::acquire();
        let mut raw = core::ptr::null_mut();
        // SAFETY: libgit2 is initialized and `raw` is writable.
        assert_eq!(unsafe { ffi::git_config_new(&mut raw) }, 0);
        // SAFETY: success returned one fresh owned config count.
        let config = unsafe { GitConfigOwned::from_raw(raw) }.unwrap();
        let snapshot = git_config_snapshot(config.as_ref()).unwrap();
        assert_ne!(snapshot.as_ref().as_ptr(), config.as_ref().as_ptr());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_config_entry
    /// A configuration entry whose storage is managed by its concrete backend.
    GitConfigEntry,
    GitConfigEntryRef,
    GitConfigEntryMut,
    ffi::git_config_entry
);

/// An owned configuration entry returned by libgit2.
pub type GitConfigEntryOwned = CBox<GitConfigEntry>;

// SAFETY: a fully formed entry is the first member of a
// `git_config_backend_entry`; `git_config_entry_free` invokes that concrete
// entry's finalizer exactly once and accepts null, although `CBox` is non-null.
ffibox::impl_dropped!(
    GitConfigEntry,
    ffi::git_config_entry,
    ffi::git_config_entry_free
);

impl<'a> GitConfigEntryRef<'a> {
    /// Field: git_config_entry.name
    /// Returns the normalized configuration name.
    #[must_use]
    pub fn name(&self) -> &'a CStr {
        // SAFETY: every live backend entry has a non-null NUL-terminated name
        // kept alive by its concrete entry owner. Raw-place projection forms
        // no reference to the C-visible entry itself.
        unsafe { CStr::from_ptr(addr_of!((*self.as_ptr()).name).read()) }
    }

    /// Field: git_config_entry.value
    /// Returns the literal value, or `None` for a valueless entry.
    #[must_use]
    pub fn value(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C-visible entry.
        let value = unsafe { addr_of!((*self.as_ptr()).value).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a non-null value is a NUL-terminated string kept alive
            // by the concrete entry owner and bounded by this handle's borrow.
            Some(unsafe { CStr::from_ptr(value) })
        }
    }

    /// Field: git_config_entry.level
    /// Returns the validated source level of this entry.
    #[must_use]
    pub fn level(&self) -> Option<GitConfigLevel> {
        // SAFETY: this live handle permits a raw-place scalar read without
        // forming a reference to the C-visible entry.
        let level = unsafe { addr_of!((*self.as_ptr()).level).read() };
        GitConfigLevel::from_raw(level)
    }

    /// Field: git_config_entry.include_depth
    /// Returns the include nesting depth at which this entry was read.
    #[must_use]
    pub fn include_depth(&self) -> u32 {
        // SAFETY: this live handle permits a raw-place scalar read without
        // forming a reference to the C-visible entry.
        unsafe { addr_of!((*self.as_ptr()).include_depth).read() }
    }

    /// Field: git_config_entry.origin_path
    /// Returns the optional path from which this entry was read.
    #[must_use]
    pub fn origin_path(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C-visible entry.
        let path = unsafe { addr_of!((*self.as_ptr()).origin_path).read() };
        if path.is_null() {
            None
        } else {
            // SAFETY: a non-null origin path is a NUL-terminated string kept
            // alive by the concrete entry owner and bounded by this borrow.
            Some(unsafe { CStr::from_ptr(path) })
        }
    }

    /// Field: git_config_entry.backend_type
    /// Returns the backend kind that supplied this entry.
    #[must_use]
    pub fn backend_type(&self) -> &'a CStr {
        // SAFETY: every live backend entry has a non-null NUL-terminated
        // backend kind kept alive by its owner. Raw-place projection forms no
        // reference to the C-visible entry itself.
        unsafe { CStr::from_ptr(addr_of!((*self.as_ptr()).backend_type).read()) }
    }
}

#[cfg(test)]
mod entry_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn config_entry_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<GitConfigEntry>(),
            size_of::<ffi::git_config_entry>()
        );
        assert_eq!(
            align_of::<GitConfigEntry>(),
            align_of::<ffi::git_config_entry>()
        );
        assert_eq!(
            size_of::<GitConfigEntryRef<'_>>(),
            size_of::<*const ffi::git_config_entry>()
        );
        assert_eq!(
            size_of::<GitConfigEntryMut<'_>>(),
            size_of::<*mut ffi::git_config_entry>()
        );
    }

    #[test]
    fn config_entry_handle_reads_required_and_optional_fields() {
        let mut raw = ffi::git_config_entry {
            name: c"core.bare".as_ptr(),
            value: c"true".as_ptr(),
            backend_type: c"file".as_ptr(),
            origin_path: c"/repo/.git/config".as_ptr(),
            include_depth: 2,
            level: GitConfigLevel::LOCAL.as_raw(),
        };

        // SAFETY: `raw` and all string literals remain live for the handle's
        // complete use, and this is the only handle accessing the local value.
        let entry = unsafe { GitConfigEntryMut::from_ptr(&raw mut raw) }
            .expect("the address of a local entry is non-null");
        let shared = entry.as_ref();
        assert_eq!(shared.name(), c"core.bare");
        assert_eq!(shared.value(), Some(c"true"));
        assert_eq!(shared.backend_type(), c"file");
        assert_eq!(shared.origin_path(), Some(c"/repo/.git/config"));
        assert_eq!(shared.include_depth(), 2);
        assert_eq!(shared.level(), Some(GitConfigLevel::LOCAL));

        raw.value = core::ptr::null();
        raw.origin_path = core::ptr::null();
        raw.level = 1;
        // SAFETY: the prior handle is no longer used, and `raw` remains live
        // with valid null optional fields and an initialized integer level.
        let entry = unsafe { GitConfigEntryRef::from_ptr(&raw mut raw) }
            .expect("the address of a local entry is non-null");
        assert_eq!(entry.value(), None);
        assert_eq!(entry.origin_path(), None);
        assert_eq!(entry.level(), None);
    }
}

fn config_result(status: i32, config: Option<GitConfigOwned>) -> Result<GitConfigOwned, i32> {
    if status == 0 {
        Ok(config.expect("libgit2 succeeded without returning a config"))
    } else {
        drop(config);
        Err(status)
    }
}

/// Wraps: git_config_add_file_ondisk
/// Adds a file backend without a repository-dependent include context.
pub fn git_config_add_file_ondisk(
    config: &mut GitConfigMut<'_>,
    path: &CStr,
    level: GitConfigLevel,
    force: bool,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `path` is a live C string.
    // Passing null guarantees the backend stores no repository borrow.
    let status = unsafe {
        ffi::git_config_add_file_ondisk(
            config.as_mut_ptr(),
            path.as_ptr(),
            level.as_raw(),
            core::ptr::null(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_add_file_ondisk
/// Adds a file backend that retains `repo` for conditional-include matching.
///
/// # Safety
///
/// `repo` must remain alive until `config` and every config sharing the added
/// backend have been freed. The existing config type cannot express a lifetime
/// introduced by mutating it in place.
pub unsafe fn git_config_add_file_ondisk_with_repository(
    config: &mut GitConfigMut<'_>,
    path: &CStr,
    level: GitConfigLevel,
    repo: crate::repository::GitRepositoryRef<'_>,
    force: bool,
) -> Result<(), i32> {
    // SAFETY: typed handles and `path` are live for the call; the caller
    // upholds the backend's longer-lived repository-pointer obligation.
    let status = unsafe {
        ffi::git_config_add_file_ondisk(
            config.as_mut_ptr(),
            path.as_ptr(),
            level.as_raw(),
            repo.as_ptr(),
            i32::from(force),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_delete_entry
/// Deletes the highest-priority writable value for `name`.
pub fn git_config_delete_entry(config: &mut GitConfigMut<'_>, name: &CStr) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is a live C string.
    let status = unsafe { ffi::git_config_delete_entry(config.as_mut_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_delete_multivar
/// Deletes multivar values matching `regexp`.
pub fn git_config_delete_multivar(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    regexp: &CStr,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and both strings are live.
    let status = unsafe {
        ffi::git_config_delete_multivar(config.as_mut_ptr(), name.as_ptr(), regexp.as_ptr())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

fn find_config_path(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    call: unsafe extern "C" fn(*mut ffi::git_buf) -> core::ffi::c_int,
) -> Result<(), i32> {
    // SAFETY: `out` is live and exclusive and `call` is one of the scheduled
    // libgit2 path-finder functions with this exact output contract.
    let status = unsafe { call(out.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_find_global
/// Writes the discovered global configuration path.
pub fn git_config_find_global(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_global)
}

/// Wraps: git_config_find_system
/// Writes the discovered system configuration path.
pub fn git_config_find_system(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_system)
}

/// Wraps: git_config_find_xdg
/// Writes the discovered XDG configuration path.
pub fn git_config_find_xdg(out: &mut crate::api::buffer::GitBufMut<'_>) -> Result<(), i32> {
    find_config_path(out, ffi::git_config_find_xdg)
}

/// Wraps: git_config_get_bool
/// Reads and parses a boolean configuration value.
pub fn git_config_get_bool(config: GitConfigRef<'_>, name: &CStr) -> Result<bool, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_bool(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 {
        Ok(out != 0)
    } else {
        Err(status)
    }
}

/// Wraps: git_config_get_int32
/// Reads and parses a 32-bit integer configuration value.
pub fn git_config_get_int32(config: GitConfigRef<'_>, name: &CStr) -> Result<i32, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_int32(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_get_int64
/// Reads and parses a 64-bit integer configuration value.
pub fn git_config_get_int64(config: GitConfigRef<'_>, name: &CStr) -> Result<i64, i32> {
    let mut out = 0;
    // SAFETY: the config and string are live and `out` is writable.
    let status = unsafe { ffi::git_config_get_int64(&mut out, config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_get_path
/// Expands and writes a path-valued configuration entry.
pub fn git_config_get_path(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    config: GitConfigRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: both handles and `name` are live, and `out` is exclusive.
    let status =
        unsafe { ffi::git_config_get_path(out.as_mut_ptr(), config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_get_string
/// Borrows a string from a read-only snapshot config.
pub fn git_config_get_string<'a>(config: GitConfigRef<'a>, name: &CStr) -> Result<&'a CStr, i32> {
    let mut out = core::ptr::null();
    // SAFETY: the config and name are live and `out` is writable. Success
    // returns a string retained by the snapshot config.
    let status = unsafe { ffi::git_config_get_string(&mut out, config.as_ptr(), name.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    debug_assert!(!out.is_null());
    // SAFETY: success returns a non-null NUL-terminated string retained by `config`.
    Ok(unsafe { CStr::from_ptr(out) })
}

/// Wraps: git_config_get_string_buf
/// Copies a string-valued entry into `out`.
pub fn git_config_get_string_buf(
    out: &mut crate::api::buffer::GitBufMut<'_>,
    config: GitConfigRef<'_>,
    name: &CStr,
) -> Result<(), i32> {
    // SAFETY: both handles and `name` are live, and `out` is exclusive.
    let status =
        unsafe { ffi::git_config_get_string_buf(out.as_mut_ptr(), config.as_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_new
/// Allocates an empty configuration object.
pub fn git_config_new() -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable owner output slot.
    let status = unsafe { ffi::git_config_new(&mut out) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_default
/// Opens the default prioritized configuration stack.
pub fn git_config_open_default() -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable owner output slot.
    let status = unsafe { ffi::git_config_open_default(&mut out) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_global
/// Opens the writable global or XDG backend from `config`.
pub fn git_config_open_global(config: GitConfigRef<'_>) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `config` is live and source inspection shows it is only read;
    // `out` is writable. The returned config refcounts its backend instance.
    let status = unsafe { ffi::git_config_open_global(&mut out, config.as_ptr().cast_mut()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_level
/// Opens a focused configuration object for one priority level.
pub fn git_config_open_level(
    parent: GitConfigRef<'_>,
    level: GitConfigLevel,
) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `parent` is live, `out` is writable, and a checked level is passed.
    let status = unsafe { ffi::git_config_open_level(&mut out, parent.as_ptr(), level.as_raw()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_open_ondisk
/// Opens one on-disk configuration file.
pub fn git_config_open_ondisk(path: &CStr) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `path` is live and `out` is writable.
    let status = unsafe { ffi::git_config_open_ondisk(&mut out, path.as_ptr()) };
    // SAFETY: `out` is null or the complete config count produced by this FFI
    // call. Adopting it here keeps the raw ownership conversion at the seam.
    let out = unsafe { GitConfigOwned::from_raw(out) };
    config_result(status, out)
}

/// Wraps: git_config_set_bool
/// Stores a boolean in the highest-priority writable backend.
pub fn git_config_set_bool(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: bool,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status =
        unsafe { ffi::git_config_set_bool(config.as_mut_ptr(), name.as_ptr(), i32::from(value)) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_int32
/// Stores a 32-bit integer in the highest-priority writable backend.
pub fn git_config_set_int32(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: i32,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status = unsafe { ffi::git_config_set_int32(config.as_mut_ptr(), name.as_ptr(), value) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_int64
/// Stores a 64-bit integer in the highest-priority writable backend.
pub fn git_config_set_int64(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: i64,
) -> Result<(), i32> {
    // SAFETY: the config is live and exclusive and `name` is live.
    let status = unsafe { ffi::git_config_set_int64(config.as_mut_ptr(), name.as_ptr(), value) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_multivar
/// Replaces every value matching `regexp` in the highest-priority writable
/// backend. Libgit2 reads all strings only for the duration of the call.
pub fn git_config_set_multivar(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    regexp: &CStr,
    value: &CStr,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies a live config and all three
    // arguments are live NUL-terminated strings that libgit2 does not retain.
    let status = unsafe {
        ffi::git_config_set_multivar(
            config.as_mut_ptr(),
            name.as_ptr(),
            regexp.as_ptr(),
            value.as_ptr(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_set_string
/// Copies a string into the highest-priority writable backend.
pub fn git_config_set_string(
    config: &mut GitConfigMut<'_>,
    name: &CStr,
    value: &CStr,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies a live config and both strings
    // remain live and NUL-terminated throughout this non-retaining call.
    let status =
        unsafe { ffi::git_config_set_string(config.as_mut_ptr(), name.as_ptr(), value.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_snapshot
/// Creates an independently owned, read-only snapshot of `config`.
pub fn git_config_snapshot(config: GitConfigRef<'_>) -> Result<GitConfigOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and `config` is a live shared handle. The C
    // declaration predates const-correctness but snapshotting only reads the
    // source's reader vector and asks each backend for a snapshot. The
    // snapshot backend does record the source backend pointer, but consults it
    // solely in the `open` that `git_config_add_backend` performs inside this
    // call, and copies out every entry there; the result is independent of the
    // source once this call returns.
    let status = unsafe { ffi::git_config_snapshot(&mut out, config.as_ptr().cast_mut()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns one fresh owned config count, released by the
    // `GitConfig` drop contract.
    unsafe { GitConfigOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_config_get_entry
/// Looks up a configuration entry and returns its independently releasable owner.
pub fn git_config_get_entry(
    config: GitConfigRef<'_>,
    name: &CStr,
) -> Result<GitConfigEntryOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable, the config and name are live for the call,
    // and success transfers one backend-entry owner to the caller.
    let status = unsafe { ffi::git_config_get_entry(&mut out, config.as_ptr(), name.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a fully formed caller-owned config entry.
    unsafe { GitConfigEntryOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_config_iterator_glob_new
/// Creates an iterator over entries whose normalized names match `regexp`.
pub fn git_config_iterator_glob_new<'config>(
    config: GitConfigRef<'config>,
    regexp: Option<&CStr>,
) -> Result<GitConfigIteratorOwned<'config>, i32> {
    let mut out = core::ptr::null_mut();
    let regexp = regexp.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the output is writable, the config and optional expression are
    // live, and a transferred iterator is adopted before leaving the FFI seam.
    // The returned owner retains the config borrow that C stores.
    let (status, inner) = unsafe {
        let status = ffi::git_config_iterator_glob_new(&mut out, config.as_ptr(), regexp);
        (
            status,
            crate::sys::config::GitConfigIteratorOwned::from_raw(out),
        )
    };
    iterator_result(status, inner)
}

/// Wraps: git_config_iterator_new
/// Creates an iterator over every entry in `config`.
pub fn git_config_iterator_new<'config>(
    config: GitConfigRef<'config>,
) -> Result<GitConfigIteratorOwned<'config>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: the output is writable and a transferred iterator is adopted
    // before leaving the FFI seam. `config` remains borrowed for the lifetime
    // carried by the returned iterator owner.
    let (status, inner) = unsafe {
        let status = ffi::git_config_iterator_new(&mut out, config.as_ptr());
        (
            status,
            crate::sys::config::GitConfigIteratorOwned::from_raw(out),
        )
    };
    iterator_result(status, inner)
}

/// Wraps: git_config_multivar_iterator_new
/// Creates an iterator over values of `name`, optionally filtered by `regexp`.
pub fn git_config_multivar_iterator_new<'config>(
    config: GitConfigRef<'config>,
    name: &CStr,
    regexp: Option<&CStr>,
) -> Result<GitConfigIteratorOwned<'config>, i32> {
    let mut out = core::ptr::null_mut();
    let regexp = regexp.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the output is writable, all strings are live for the call, and
    // a transferred iterator is adopted before leaving the FFI seam. The
    // returned iterator's stored config pointer is lifetime-bound here.
    let (status, inner) = unsafe {
        let status =
            ffi::git_config_multivar_iterator_new(&mut out, config.as_ptr(), name.as_ptr(), regexp);
        (
            status,
            crate::sys::config::GitConfigIteratorOwned::from_raw(out),
        )
    };
    iterator_result(status, inner)
}

/// Wraps: git_config_next
/// Advances an iterator and borrows its current public entry.
pub fn git_config_next<'iter>(
    iterator: &'iter mut crate::sys::config::GitConfigIteratorMut<'_>,
) -> Result<GitConfigEntryRef<'iter>, i32> {
    let mut entry = core::ptr::null_mut();
    // SAFETY: the output is writable and the iterator is exclusively borrowed
    // for the duration of the call and of the returned transient entry.
    let status = unsafe { ffi::git_config_next(&mut entry, iterator.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success publishes the iterator-managed current entry. Its handle
    // is tied to the exclusive iterator reborrow, preventing invalidation.
    unsafe { GitConfigEntryRef::from_ptr(entry) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

#[cfg(test)]
mod scheduled_iterator_tests {
    use super::*;

    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: initialization is process-global and refcounted.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances this guard's successful initialization count.
            unsafe { ffi::git_libgit2_shutdown() };
        }
    }

    #[test]
    fn empty_config_iterator_is_owned_and_reports_completion() {
        let _init = Libgit2Init::acquire();
        let config = crate::config::git_config_new().unwrap();
        let mut iterator = git_config_iterator_new(config.as_ref()).unwrap();
        assert_eq!(
            git_config_next(&mut iterator.as_mut()).err(),
            Some(ffi::git_error_code_GIT_ITEROVER)
        );
        drop(iterator);
        drop(config);
    }
}

/// A configuration transaction tied to the configuration it has locked.
pub struct ConfigTransaction<'config> {
    inner: crate::transaction::GitTransactionOwned,
    _config: core::marker::PhantomData<GitConfigRef<'config>>,
}

impl ConfigTransaction<'_> {
    /// Borrows the transaction exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> crate::transaction::GitTransactionMut<'_> {
        self.inner.as_mut()
    }
}

/// Wraps: git_config_lock
/// Locks the writable backend and returns a transaction tied to `config`.
pub fn git_config_lock<'config>(
    config: &'config mut GitConfigMut<'_>,
) -> Result<ConfigTransaction<'config>, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is writable and the returned transaction carries the
    // exclusive borrow of the config pointer retained until it is freed.
    let status = unsafe { ffi::git_config_lock(&mut out, config.as_mut_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one complete configuration transaction.
    let inner = unsafe { crate::transaction::GitTransactionOwned::from_raw(out) }
        .ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(ConfigTransaction {
        inner,
        _config: core::marker::PhantomData,
    })
}

/// Wraps: git_config_parse_path
/// Expands a configuration path into a newly owned buffer.
pub fn git_config_parse_path(
    value: &CStr,
) -> Result<ffibox::CVal<crate::api::buffer::GitBuf>, i32> {
    let mut out = crate::api::buffer::GitBuf::new();
    // SAFETY: the output header is exclusively writable and `value` is a live
    // C string used only for this parse.
    let status = unsafe { ffi::git_config_parse_path(out.as_mut().as_mut_ptr(), value.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_set_writeorder
/// Reorders writable backends according to `levels`.
pub fn git_config_set_writeorder(
    config: &mut GitConfigMut<'_>,
    levels: &[GitConfigLevel],
) -> Result<(), i32> {
    if levels.len() >= i32::MAX as usize {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    // SAFETY: `GitConfigLevel` is transparent over the C scalar; the slice is
    // readable for the call, and the exclusive config handle permits sorting.
    let status = unsafe {
        ffi::git_config_set_writeorder(
            config.as_mut_ptr(),
            levels.as_ptr().cast_mut().cast(),
            levels.len(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Copies borrowed mappings into the contiguous C array both lookups take.
///
/// A [`GitConfigmapType::String`] entry must carry a match string: libgit2
/// reaches that case with a bare `strcasecmp(value, m->str_match)` and never
/// null-checks `str_match`, so a null one faults inside C instead of failing
/// the lookup. [`GitConfigmap::new`] leaves that field null, so safe callers
/// can build exactly such a value; rejecting it here with `GIT_EINVALID` is
/// what keeps the two mapped lookups safe.
///
/// [`GitConfigmapType::String`]: crate::api::config::GitConfigmapType::String
/// [`GitConfigmap::new`]: crate::api::config::GitConfigmap::new
fn raw_configmaps(
    maps: &[crate::api::config::GitConfigmapRef<'_>],
) -> Result<Vec<ffi::git_configmap>, i32> {
    maps.iter()
        .map(|map| {
            let kind = map.kind().map_err(|_| ffi::git_error_code_GIT_EINVALID)?;
            let str_match = map.str_match();
            if kind == crate::api::config::GitConfigmapType::String && str_match.is_none() {
                return Err(ffi::git_error_code_GIT_EINVALID);
            }
            Ok(ffi::git_configmap {
                type_: kind.into(),
                str_match: str_match.map_or(core::ptr::null(), CStr::as_ptr),
                map_value: map.map_value(),
            })
        })
        .collect()
}

/// Wraps: git_config_get_mapped
/// Reads and maps a named configuration value.
///
/// A string mapping carrying no match string is rejected with `GIT_EINVALID`
/// before the call: libgit2 would hand its null pointer straight to
/// `strcasecmp`.
pub fn git_config_get_mapped(
    config: GitConfigRef<'_>,
    name: &CStr,
    maps: &[crate::api::config::GitConfigmapRef<'_>],
) -> Result<i32, i32> {
    let maps = raw_configmaps(maps)?;
    let mut out = 0;
    // SAFETY: output, config, name and the contiguous copied mapping array are
    // live for the call; mapping string pointers remain borrowed from inputs.
    let status = unsafe {
        ffi::git_config_get_mapped(
            &mut out,
            config.as_ptr(),
            name.as_ptr(),
            maps.as_ptr(),
            maps.len(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_lookup_map_value
/// Maps a supplied textual value through `maps`.
///
/// A string mapping carrying no match string is rejected with `GIT_EINVALID`
/// before the call: libgit2 would hand its null pointer straight to
/// `strcasecmp`.
pub fn git_config_lookup_map_value(
    maps: &[crate::api::config::GitConfigmapRef<'_>],
    value: &CStr,
) -> Result<i32, i32> {
    let maps = raw_configmaps(maps)?;
    let mut out = 0;
    // SAFETY: output, value and the contiguous copied mapping array are live
    // for this non-retaining lookup.
    let status = unsafe {
        ffi::git_config_lookup_map_value(&mut out, maps.as_ptr(), maps.len(), value.as_ptr())
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_config_foreach
/// Visits every configuration entry.
pub fn git_config_foreach<C>(config: GitConfigRef<'_>, callback: &mut C) -> Result<(), i32>
where
    C: crate::api::config::GitConfigForeachCallback,
{
    // SAFETY: the config and callback remain live for this synchronous walk;
    // the callback/payload types match and neither pointer is retained.
    let status = unsafe {
        ffi::git_config_foreach(
            config.as_ptr(),
            Some(config_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_foreach_match
/// Visits configuration entries whose normalized names match `regexp`.
pub fn git_config_foreach_match<C>(
    config: GitConfigRef<'_>,
    regexp: Option<&CStr>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::api::config::GitConfigForeachCallback,
{
    // SAFETY: all inputs remain live for the synchronous traversal; a null
    // regexp selects every entry, and no pointer is retained.
    let status = unsafe {
        ffi::git_config_foreach_match(
            config.as_ptr(),
            regexp.map_or(core::ptr::null(), CStr::as_ptr),
            Some(config_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_get_multivar_foreach
/// Visits every value for `name` that matches the optional value expression.
pub fn git_config_get_multivar_foreach<C>(
    config: GitConfigRef<'_>,
    name: &CStr,
    regexp: Option<&CStr>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::api::config::GitConfigForeachCallback,
{
    // SAFETY: all borrowed inputs remain live for the synchronous traversal;
    // the callback/payload types match and no pointer is retained.
    let status = unsafe {
        ffi::git_config_get_multivar_foreach(
            config.as_ptr(),
            name.as_ptr(),
            regexp.map_or(core::ptr::null(), CStr::as_ptr),
            Some(config_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_config_backend_foreach_match
/// Visits backend entries whose normalized names match `regexp`.
pub fn git_config_backend_foreach_match<C>(
    backend: &mut crate::sys::config::GitConfigBackendMut<'_>,
    regexp: Option<&CStr>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::api::config::GitConfigForeachCallback,
{
    // SAFETY: the backend is exclusively borrowed for iterator creation and
    // advancement; the optional expression and callback remain live for this
    // synchronous traversal, and neither callback pointer is retained.
    let status = unsafe {
        ffi::git_config_backend_foreach_match(
            backend.as_mut_ptr(),
            regexp.map_or(core::ptr::null(), CStr::as_ptr),
            Some(config_foreach_trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_backend_foreach_tests {
    use super::*;

    struct Noop;

    impl crate::api::config::GitConfigForeachCallback for Noop {
        fn call(&mut self, _entry: GitConfigEntryRef<'_>) -> i32 {
            0
        }
    }

    #[test]
    fn backend_traversal_requires_an_exclusive_backend_and_typed_callback() {
        let _: fn(
            &mut crate::sys::config::GitConfigBackendMut<'_>,
            Option<&CStr>,
            &mut Noop,
        ) -> Result<(), i32> = git_config_backend_foreach_match::<Noop>;
    }
}

#[cfg(test)]
mod foreach_tests {
    use super::*;

    #[test]
    fn foreach_on_an_empty_config_invokes_no_callback() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced after all owners created by the test are dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let config = git_config_new().expect("empty config allocation");
        let mut calls = 0;
        git_config_foreach(config.as_ref(), &mut |_entry: GitConfigEntryRef<'_>| {
            calls += 1;
            0
        })
        .expect("empty traversal succeeds");
        assert_eq!(calls, 0);
        drop(config);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

#[cfg(test)]
mod scheduled_configmap_tests {
    use super::*;
    use crate::api::config::{GitConfigmap, GitConfigmapType};

    /// Holds one libgit2 initialization count for the duration of a test.
    ///
    /// A failed mapping reaches `git_error_set`, which writes through
    /// thread-local error state that only initialization creates; without
    /// this guard the failing lookups below are wild writes rather than
    /// errors.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and reference
            // counted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the initialization this guard represents,
            // after every libgit2 owner in the test has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A string mapping with no match string is what libgit2 would hand to
    /// `strcasecmp` as a null pointer, so both mapped lookups must refuse it
    /// before the call rather than reproduce the fault.
    #[test]
    fn a_string_mapping_without_a_match_string_is_rejected() {
        let mapping = GitConfigmap::new(GitConfigmapType::String, 7);
        let maps = [mapping.as_ref()];
        assert_eq!(
            git_config_lookup_map_value(&maps, c"anything"),
            Err(ffi::git_error_code_GIT_EINVALID)
        );
    }

    #[test]
    fn a_string_mapping_with_a_match_string_is_compared_case_insensitively() {
        let _libgit2 = Libgit2Init::acquire();
        let mut mapping = GitConfigmap::new(GitConfigmapType::String, 7);
        // SAFETY: this static string outlives every use of `mapping`.
        unsafe { mapping.as_mut().set_borrowed_str_match(Some(c"AlWaYs")) };
        let maps = [mapping.as_ref()];
        assert_eq!(git_config_lookup_map_value(&maps, c"always"), Ok(7));
        assert_eq!(
            git_config_lookup_map_value(&maps, c"never"),
            Err(ffi::git_error_code_GIT_ERROR)
        );
    }

    /// The other kinds never read `str_match`, so leaving it null is fine.
    #[test]
    fn non_string_mappings_do_not_need_a_match_string() {
        let _libgit2 = Libgit2Init::acquire();

        let boolean = GitConfigmap::new(GitConfigmapType::False, 3);
        let maps = [boolean.as_ref()];
        assert_eq!(git_config_lookup_map_value(&maps, c"false"), Ok(3));

        // An integer mapping yields the parsed value rather than its own, and
        // is tried alone here: libgit2 parses any integer as a true boolean,
        // so a preceding `True` mapping would have claimed this value first.
        let integer = GitConfigmap::new(GitConfigmapType::Int32, 0);
        let maps = [integer.as_ref()];
        assert_eq!(git_config_lookup_map_value(&maps, c"42"), Ok(42));
    }

    /// The named-value form maps through the same array, so it inherits the
    /// same rejection without reaching libgit2 at all.
    #[test]
    fn the_named_lookup_rejects_the_same_mapping() {
        let _libgit2 = Libgit2Init::acquire();
        let config = git_config_new().expect("empty config allocation");
        let mapping = GitConfigmap::new(GitConfigmapType::String, 7);
        let maps = [mapping.as_ref()];
        assert_eq!(
            git_config_get_mapped(config.as_ref(), c"core.autocrlf", &maps),
            Err(ffi::git_error_code_GIT_EINVALID)
        );
    }
}
