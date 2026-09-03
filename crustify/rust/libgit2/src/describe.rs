//! Safe wrappers for libgit2 describe APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::{CBox, CVal, define_ctype, impl_dropped};

use crate::api::buffer::GitBuf;
use crate::ffi;
use crate::object::GitObjectRef;
use crate::repository::GitRepositoryRef;

define_ctype!(
    /// Formatting options consumed by libgit2 describe operations.
    ///
    /// Wraps: git_describe_format_options
    DescribeFormatOptions,
    DescribeFormatOptionsRef,
    DescribeFormatOptionsMut,
    ffi::git_describe_format_options
);

impl<'a> DescribeFormatOptionsRef<'a> {
    /// Field: git_describe_format_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_describe_format_options.dirty_suffix
    /// Returns the optional suffix borrowed by this options value.
    #[must_use]
    pub fn dirty_suffix(&self) -> Option<&'a CStr> {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the pointer field without forming a
        // reference to C-owned memory.
        let suffix = unsafe { addr_of!((*self.as_ptr()).dirty_suffix).read() };

        if suffix.is_null() {
            None
        } else {
            // SAFETY: a live `git_describe_format_options` requires its
            // non-null `dirty_suffix` to point to a NUL-terminated string that
            // remains valid while the options value is usable. The result is
            // bounded by the handle's borrow of that value.
            Some(unsafe { CStr::from_ptr(suffix) })
        }
    }

    /// Field: git_describe_format_options.always_use_long_format
    /// Returns whether the long form is requested for exact matches.
    #[must_use]
    pub fn always_use_long_format(&self) -> bool {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).always_use_long_format).read() != 0 }
    }

    /// Field: git_describe_format_options.abbreviated_size
    /// Returns the lower bound for abbreviated object identifiers.
    #[must_use]
    pub fn abbreviated_size(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).abbreviated_size).read() }
    }
}

impl DescribeFormatOptionsMut<'_> {
    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores an optional borrowed suffix.
    ///
    /// # Safety
    ///
    /// A non-null `suffix` must remain alive and NUL-terminated for every
    /// later use of the options value, including uses after this handle is
    /// released.
    pub unsafe fn set_dirty_suffix(&mut self, suffix: Option<&CStr>) {
        let suffix = suffix.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // `addr_of_mut!` forms no reference to C-owned memory, and the caller
        // upholds the stored pointer's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).dirty_suffix).write(suffix) }
    }

    /// Clears the optional borrowed suffix.
    pub fn clear_dirty_suffix(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime
        // obligation.
        unsafe { self.set_dirty_suffix(None) }
    }

    /// Selects whether exact matches use the long format.
    pub fn set_always_use_long_format(&mut self, always: bool) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).always_use_long_format).write(i32::from(always))
        }
    }

    /// Sets the lower bound for abbreviated object identifiers.
    pub fn set_abbreviated_size(&mut self, size: u32) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).abbreviated_size).write(size) }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn describe_format_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<DescribeFormatOptions>(),
            size_of::<ffi::git_describe_format_options>()
        );
        assert_eq!(
            align_of::<DescribeFormatOptions>(),
            align_of::<ffi::git_describe_format_options>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_every_field() {
        let mut options = DescribeFormatOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_describe_format_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above, and this is its only active handle.
        let mut options = unsafe { DescribeFormatOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        options.set_abbreviated_size(12);
        options.set_always_use_long_format(true);
        // SAFETY: this string literal has static storage and remains valid for
        // every later use of the stack options value.
        unsafe { options.set_dirty_suffix(Some(c"-dirty")) };

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.abbreviated_size(), 12);
        assert!(shared.always_use_long_format());
        assert_eq!(shared.dirty_suffix(), Some(c"-dirty"));

        options.clear_dirty_suffix();
        assert_eq!(options.as_ref().dirty_suffix(), None);
    }
}

define_ctype!(
    /// Wraps: git_describe_options
    /// Options controlling which references a describe operation considers.
    DescribeOptions,
    DescribeOptionsRef,
    DescribeOptionsMut,
    ffi::git_describe_options
);

impl<'a> DescribeOptionsRef<'a> {
    /// Field: git_describe_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads the initialized scalar
        // without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_describe_options.show_commit_oid_as_fallback
    /// Returns whether a full object ID should be used when no reference matches.
    #[must_use]
    pub fn show_commit_oid_as_fallback(&self) -> bool {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads the initialized scalar
        // without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).show_commit_oid_as_fallback).read() != 0 }
    }

    /// Field: git_describe_options.only_follow_first_parent
    /// Returns whether traversal follows only first-parent ancestry.
    #[must_use]
    pub fn only_follow_first_parent(&self) -> bool {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads the initialized scalar
        // without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).only_follow_first_parent).read() != 0 }
    }

    /// Field: git_describe_options.pattern
    /// Returns the optional caller-owned match pattern.
    #[must_use]
    pub fn pattern(&self) -> Option<&'a CStr> {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads its pointer field without
        // forming a reference to C-owned memory.
        let pattern = unsafe { addr_of!((*self.as_ptr()).pattern).read() };

        if pattern.is_null() {
            None
        } else {
            // SAFETY: a valid options value requires a non-null `pattern` to
            // address a NUL-terminated string for the value's usable
            // lifetime. The returned borrow is bounded by this handle.
            Some(unsafe { CStr::from_ptr(pattern) })
        }
    }

    /// Field: git_describe_options.describe_strategy
    /// Returns the checked reference lookup strategy.
    pub fn describe_strategy(
        &self,
    ) -> Result<crate::api::describe::GitDescribeStrategy, ffi::git_describe_strategy_t> {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads the initialized scalar
        // without forming a reference to C-owned memory.
        let raw = unsafe { addr_of!((*self.as_ptr()).describe_strategy).read() };
        crate::api::describe::GitDescribeStrategy::from_raw(raw).ok_or(raw)
    }

    /// Field: git_describe_options.max_candidates_tags
    /// Returns the maximum number of candidate tags to consider.
    #[must_use]
    pub fn max_candidates_tags(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and raw-place projection reads the initialized scalar
        // without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).max_candidates_tags).read() }
    }
}

impl DescribeOptionsMut<'_> {
    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: this exclusive handle permits mutation, and raw-place
        // projection writes the scalar without forming a reference to
        // C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects whether a full object ID is used when no reference matches.
    pub fn set_show_commit_oid_as_fallback(&mut self, enabled: bool) {
        // SAFETY: this exclusive handle permits mutation, and raw-place
        // projection writes the scalar without forming a reference to
        // C-owned memory.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).show_commit_oid_as_fallback).write(i32::from(enabled))
        }
    }

    /// Selects whether traversal follows only first-parent ancestry.
    pub fn set_only_follow_first_parent(&mut self, enabled: bool) {
        // SAFETY: this exclusive handle permits mutation, and raw-place
        // projection writes the scalar without forming a reference to
        // C-owned memory.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).only_follow_first_parent).write(i32::from(enabled))
        }
    }

    /// Stores an optional caller-owned match pattern.
    ///
    /// # Safety
    ///
    /// A non-null `pattern` must remain alive and NUL-terminated for every
    /// later use of the options value, including uses after this handle is
    /// released.
    pub unsafe fn set_pattern(&mut self, pattern: Option<&CStr>) {
        let pattern = pattern.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits mutation, raw-place
        // projection forms no reference to C-owned memory, and the caller
        // upholds the stored pointer's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).pattern).write(pattern) }
    }

    /// Clears the optional match pattern.
    pub fn clear_pattern(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime
        // obligation.
        unsafe { self.set_pattern(None) }
    }

    /// Sets the reference lookup strategy.
    pub fn set_describe_strategy(&mut self, strategy: crate::api::describe::GitDescribeStrategy) {
        // SAFETY: this exclusive handle permits mutation, and raw-place
        // projection writes the checked scalar without forming a reference to
        // C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).describe_strategy).write(strategy.as_raw()) }
    }

    /// Sets the maximum number of candidate tags to consider.
    pub fn set_max_candidates_tags(&mut self, maximum: u32) {
        // SAFETY: this exclusive handle permits mutation, and raw-place
        // projection writes the scalar without forming a reference to
        // C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).max_candidates_tags).write(maximum) }
    }
}

define_ctype!(
    /// Wraps: git_describe_result
    /// An opaque result produced by a libgit2 describe operation.
    ///
    /// Owned pointers use [`ffibox::CBox<DescribeResult>`] and are released
    /// with `git_describe_result_free`. A result borrows its repository;
    /// construction wrappers must keep that repository alive while the result
    /// can be formatted.
    DescribeResult,
    DescribeResultRef,
    DescribeResultMut,
    ffi::git_describe_result
);

// SAFETY: `git_describe_result_free` is the public destructor for a complete
// libgit2-allocated result and accepts null, although `CDropped` supplies a
// live non-null allocation. `DescribeResult` is transparent over the
// corresponding opaque bindgen type.
impl_dropped!(
    DescribeResult,
    ffi::git_describe_result,
    ffi::git_describe_result_free
);

#[cfg(test)]
mod describe_type_tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CBox, CCell, CDropped};

    use super::*;

    #[test]
    fn describe_options_preserve_layout_and_access_every_field() {
        assert_eq!(
            size_of::<DescribeOptions>(),
            size_of::<ffi::git_describe_options>()
        );
        assert_eq!(
            align_of::<DescribeOptions>(),
            align_of::<ffi::git_describe_options>()
        );

        let mut options = DescribeOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_describe_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above, and this is its only active handle.
        let mut options = unsafe { DescribeOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        options.set_max_candidates_tags(20);
        options.set_describe_strategy(crate::api::describe::GitDescribeStrategy::ALL);
        options.set_only_follow_first_parent(true);
        options.set_show_commit_oid_as_fallback(true);
        // SAFETY: this string literal has static storage and remains valid for
        // every later use of the stack options value.
        unsafe { options.set_pattern(Some(c"release-*")) };

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.max_candidates_tags(), 20);
        assert_eq!(
            shared.describe_strategy(),
            Ok(crate::api::describe::GitDescribeStrategy::ALL)
        );
        assert!(shared.only_follow_first_parent());
        assert!(shared.show_commit_oid_as_fallback());
        assert_eq!(shared.pattern(), Some(c"release-*"));

        options.clear_pattern();
        assert_eq!(options.as_ref().pattern(), None);
    }

    #[test]
    fn opaque_result_has_borrowed_handles_and_a_destructor() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<DescribeResult>();
        assert_dropped::<DescribeResult>();
        assert_eq!(
            size_of::<DescribeResult>(),
            size_of::<ffi::git_describe_result>()
        );
        assert_eq!(
            align_of::<DescribeResult>(),
            align_of::<ffi::git_describe_result>()
        );
        assert_eq!(
            size_of::<DescribeResultRef<'_>>(),
            size_of::<*const ffi::git_describe_result>()
        );
        assert_eq!(
            size_of::<DescribeResultMut<'_>>(),
            size_of::<*mut ffi::git_describe_result>()
        );

        // SAFETY: the owning conversion explicitly accepts null and returns
        // `None` without adopting or dereferencing an allocation.
        assert!(unsafe { CBox::<DescribeResult>::from_raw(ptr::null_mut()) }.is_none());
    }
}

/// Wraps: git_describe_result_free
/// An owned describe result tied to the repository from which it was built.
pub struct DescribeResultOwned<'repo> {
    inner: CBox<DescribeResult>,
    repository: core::marker::PhantomData<&'repo ()>,
}

impl<'repo> DescribeResultOwned<'repo> {
    unsafe fn from_raw(raw: *mut ffi::git_describe_result) -> Option<Self> {
        // SAFETY: the caller guarantees that `raw` is null or a fresh complete
        // result whose destructor is registered above.
        let inner = unsafe { CBox::from_raw(raw) }?;
        Some(Self {
            inner,
            repository: core::marker::PhantomData,
        })
    }

    /// Borrows the result for formatting.
    #[must_use]
    pub fn as_ref(&self) -> DescribeResultRef<'_> {
        self.inner.as_ref()
    }
}

/// Wraps: git_describe_commit
/// Describes `committish` and ties the result to its repository lifetime.
pub fn git_describe_commit<'repo>(
    committish: GitObjectRef<'repo>,
    options: Option<DescribeOptionsRef<'_>>,
) -> Result<DescribeResultOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: `out` is writable, both handles are live for the call, and C
    // copies the options. The returned result only borrows the object's
    // repository, represented by `'repo` in the owning result.
    let status =
        unsafe { ffi::git_describe_commit(&mut out, committish.as_ptr().cast_mut(), options) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a fresh complete describe result.
    unsafe { DescribeResultOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_describe_format
/// Formats a describe result into a field-owning libgit2 buffer.
pub fn git_describe_format(
    out: &mut CVal<GitBuf>,
    result: DescribeResultRef<'_>,
    options: Option<DescribeFormatOptionsRef<'_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |value| value.as_ptr());
    // SAFETY: the output header is exclusively borrowed and initialized, and
    // both input handles remain live for this non-retaining call.
    let status =
        unsafe { ffi::git_describe_format(out.as_mut().as_mut_ptr(), result.as_ptr(), options) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_describe_workdir
/// Describes `HEAD` and the worktree, tying the result to `repository`.
pub fn git_describe_workdir<'repo>(
    repository: GitRepositoryRef<'repo>,
    options: Option<DescribeOptionsRef<'_>>,
) -> Result<DescribeResultOwned<'repo>, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null_mut(), |value| value.as_ptr().cast_mut());
    // SAFETY: `out` is writable and the two live handles satisfy the C call.
    // The successful result retains a non-owning repository pointer, whose
    // lifetime is carried by `DescribeResultOwned<'repo>`.
    let status =
        unsafe { ffi::git_describe_workdir(&mut out, repository.as_ptr().cast_mut(), options) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success returns a fresh complete describe result.
    unsafe { DescribeResultOwned::from_raw(out) }.ok_or(-1)
}

/// Wraps: git_describe_format_options_init
/// Initializes describe formatting options for `version`.
pub fn git_describe_format_options_init(
    options: &mut DescribeFormatOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: `options` exclusively exposes complete writable C storage.
    let status = unsafe { ffi::git_describe_format_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_describe_options_init
/// Initializes describe lookup options for `version`.
pub fn git_describe_options_init(
    options: &mut DescribeOptionsMut<'_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: `options` exclusively exposes complete writable C storage.
    let status = unsafe { ffi::git_describe_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init, RawBuf, safe_buf_bytes};

    unsafe fn raw_description(repository: *mut ffi::git_repository) -> Vec<u8> {
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut object, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut result = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_describe_commit(&mut result, object, core::ptr::null_mut()) },
            0
        );
        let mut buffer = RawBuf::new();
        assert_eq!(
            unsafe { ffi::git_describe_format(&mut buffer.0, result, core::ptr::null()) },
            0
        );
        let bytes = buffer.bytes();
        unsafe {
            ffi::git_describe_result_free(result);
            ffi::git_object_free(object);
        }
        bytes
    }

    unsafe fn raw_describe_matrix(repository: *mut ffi::git_repository) -> Vec<Vec<u8>> {
        let mut object = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut object, repository, c"HEAD".as_ptr()) },
            0
        );

        let mut observations = Vec::new();
        let cases = [
            (
                ffi::git_describe_strategy_t_GIT_DESCRIBE_DEFAULT,
                core::ptr::null(),
                0,
                0,
            ),
            (
                ffi::git_describe_strategy_t_GIT_DESCRIBE_TAGS,
                c"v*".as_ptr(),
                1,
                0,
            ),
            (
                ffi::git_describe_strategy_t_GIT_DESCRIBE_ALL,
                c"*".as_ptr(),
                1,
                0,
            ),
            (
                ffi::git_describe_strategy_t_GIT_DESCRIBE_ALL,
                c"missing-*".as_ptr(),
                1,
                1,
            ),
        ];
        for (strategy, pattern, first_parent, fallback) in cases {
            let mut options = unsafe { core::mem::zeroed::<ffi::git_describe_options>() };
            assert_eq!(
                unsafe {
                    ffi::git_describe_options_init(&mut options, ffi::GIT_DESCRIBE_OPTIONS_VERSION)
                },
                0
            );
            options.describe_strategy = strategy;
            options.pattern = pattern;
            options.only_follow_first_parent = first_parent;
            options.show_commit_oid_as_fallback = fallback;
            options.max_candidates_tags = 16;

            let mut result = core::ptr::null_mut();
            assert_eq!(
                unsafe { ffi::git_describe_commit(&mut result, object, &mut options) },
                0
            );
            let mut format = unsafe { core::mem::zeroed::<ffi::git_describe_format_options>() };
            assert_eq!(
                unsafe {
                    ffi::git_describe_format_options_init(
                        &mut format,
                        ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION,
                    )
                },
                0
            );
            format.always_use_long_format = 1;
            format.abbreviated_size = 12;
            let mut buffer = RawBuf::new();
            assert_eq!(
                unsafe { ffi::git_describe_format(&mut buffer.0, result, &format) },
                0
            );
            observations.push(buffer.bytes());
            unsafe { ffi::git_describe_result_free(result) };
        }

        std::fs::write(
            unsafe { std::ffi::CStr::from_ptr(ffi::git_repository_workdir(repository)) }
                .to_str()
                .unwrap()
                .to_owned()
                + "README.md",
            b"dirty describe worktree\n",
        )
        .unwrap();
        let mut result = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_describe_workdir(&mut result, repository, core::ptr::null_mut()) },
            0
        );
        let mut format = unsafe { core::mem::zeroed::<ffi::git_describe_format_options>() };
        assert_eq!(
            unsafe {
                ffi::git_describe_format_options_init(
                    &mut format,
                    ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION,
                )
            },
            0
        );
        format.dirty_suffix = c"-worktree".as_ptr();
        let mut buffer = RawBuf::new();
        assert_eq!(
            unsafe { ffi::git_describe_format(&mut buffer.0, result, &format) },
            0
        );
        observations.push(buffer.bytes());
        unsafe {
            ffi::git_describe_result_free(result);
            ffi::git_object_free(object);
        }
        observations
    }

    fn safe_description(repository: crate::repository::GitRepositoryMut<'_>) -> Vec<u8> {
        let object = crate::revparse::git_revparse_single(repository, c"HEAD").unwrap();
        let result = git_describe_commit(object.as_ref(), None).unwrap();
        let mut buffer = GitBuf::new();
        git_describe_format(&mut buffer, result.as_ref(), None).unwrap();
        safe_buf_bytes(buffer.as_ref())
    }

    fn safe_describe_matrix(
        mut repository: crate::repository::GitRepositoryMut<'_>,
    ) -> Vec<Vec<u8>> {
        let repository_ptr = repository.as_mut_ptr();
        let object = crate::revparse::git_revparse_single(repository, c"HEAD").unwrap();
        let cases = [
            (
                crate::api::describe::GitDescribeStrategy::DEFAULT,
                None,
                false,
            ),
            (
                crate::api::describe::GitDescribeStrategy::TAGS,
                Some(c"v*"),
                false,
            ),
            (
                crate::api::describe::GitDescribeStrategy::ALL,
                Some(c"*"),
                false,
            ),
            (
                crate::api::describe::GitDescribeStrategy::ALL,
                Some(c"missing-*"),
                true,
            ),
        ];
        let mut observations = Vec::new();
        for (strategy, pattern, fallback) in cases {
            let mut options = DescribeOptions::zeroed();
            let mut options =
                unsafe { DescribeOptionsMut::from_ptr(core::ptr::addr_of_mut!(options).cast()) }
                    .unwrap();
            git_describe_options_init(&mut options, ffi::GIT_DESCRIBE_OPTIONS_VERSION).unwrap();
            options.set_describe_strategy(strategy);
            options.set_only_follow_first_parent(true);
            options.set_show_commit_oid_as_fallback(fallback);
            options.set_max_candidates_tags(16);
            unsafe { options.set_pattern(pattern) };

            let result = git_describe_commit(object.as_ref(), Some(options.as_ref())).unwrap();
            let mut format = DescribeFormatOptions::zeroed();
            let mut format = unsafe {
                DescribeFormatOptionsMut::from_ptr(core::ptr::addr_of_mut!(format).cast())
            }
            .unwrap();
            git_describe_format_options_init(&mut format, ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION)
                .unwrap();
            format.set_always_use_long_format(true);
            format.set_abbreviated_size(12);
            let mut buffer = GitBuf::new();
            git_describe_format(&mut buffer, result.as_ref(), Some(format.as_ref())).unwrap();
            observations.push(safe_buf_bytes(buffer.as_ref()));
        }
        drop(object);
        let repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository_ptr) }.unwrap();

        let workdir = crate::repository::git_repository_workdir(repository.as_ref()).unwrap();
        std::fs::write(
            std::path::Path::new(workdir.to_str().unwrap()).join("README.md"),
            b"dirty describe worktree\n",
        )
        .unwrap();
        let result = git_describe_workdir(repository.as_ref(), None).unwrap();
        let mut format = DescribeFormatOptions::zeroed();
        let mut format =
            unsafe { DescribeFormatOptionsMut::from_ptr(core::ptr::addr_of_mut!(format).cast()) }
                .unwrap();
        git_describe_format_options_init(&mut format, ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION)
            .unwrap();
        unsafe { format.set_dirty_suffix(Some(c"-worktree")) };
        let mut buffer = GitBuf::new();
        git_describe_format(&mut buffer, result.as_ref(), Some(format.as_ref())).unwrap();
        observations.push(safe_buf_bytes(buffer.as_ref()));
        observations
    }

    #[test]
    fn io_equiv_describe_tagged_head() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("describe-raw");
        let safe = HistoryFixture::new("describe-safe");
        let raw_description = unsafe { raw_description(raw.repository.as_ptr()) };
        let safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        assert_eq!(raw_description, safe_description(safe_repository));
        assert_eq!(raw_description, b"v1.0");
    }

    #[test]
    fn io_equiv_describe_strategies_fallback_long_format_and_dirty_worktree() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("describe-matrix-raw");
        let safe = HistoryFixture::new("describe-matrix-safe");
        let raw_observations = unsafe { raw_describe_matrix(raw.repository.as_ptr()) };
        let safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        let safe_observations = safe_describe_matrix(safe_repository);
        assert_eq!(raw_observations, safe_observations);
        assert!(raw_observations.last().unwrap().ends_with(b"-worktree"));
    }
}

#[cfg(test)]
mod scheduled_initializer_tests {
    use super::*;
    #[test]
    fn public_initializers_write_current_versions() {
        let mut format = DescribeFormatOptions::zeroed();
        // SAFETY: this stack value is live and exclusively borrowed here.
        let mut format =
            unsafe { DescribeFormatOptionsMut::from_ptr(core::ptr::addr_of_mut!(format).cast()) }
                .unwrap();
        git_describe_format_options_init(&mut format, ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION)
            .unwrap();
        assert_eq!(
            format.as_ref().version(),
            ffi::GIT_DESCRIBE_FORMAT_OPTIONS_VERSION
        );
        let mut options = DescribeOptions::zeroed();
        // SAFETY: this stack value is live and exclusively borrowed here.
        let mut options =
            unsafe { DescribeOptionsMut::from_ptr(core::ptr::addr_of_mut!(options).cast()) }
                .unwrap();
        git_describe_options_init(&mut options, ffi::GIT_DESCRIBE_OPTIONS_VERSION).unwrap();
        assert_eq!(
            options.as_ref().version(),
            ffi::GIT_DESCRIBE_OPTIONS_VERSION
        );
    }
}
