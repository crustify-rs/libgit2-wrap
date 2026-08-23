//! Safe wrappers for libgit2 remote APIs.

use core::ptr::{NonNull, addr_of};

use ffibox::{CBox, CCloned};

use crate::ffi;
use crate::oid::OidRef;

/// Wraps: git_fetch_prune_t
/// Controls whether a fetch prunes remote-tracking references.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitFetchPrune {
    /// Use the repository or remote configuration.
    #[default]
    Unspecified = ffi::git_fetch_prune_t_GIT_FETCH_PRUNE_UNSPECIFIED,
    /// Force pruning on.
    Prune = ffi::git_fetch_prune_t_GIT_FETCH_PRUNE,
    /// Force pruning off.
    NoPrune = ffi::git_fetch_prune_t_GIT_FETCH_NO_PRUNE,
}

impl From<GitFetchPrune> for ffi::git_fetch_prune_t {
    fn from(value: GitFetchPrune) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_fetch_prune_t> for GitFetchPrune {
    type Error = ffi::git_fetch_prune_t;

    fn try_from(value: ffi::git_fetch_prune_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_fetch_prune_t_GIT_FETCH_PRUNE_UNSPECIFIED => Ok(Self::Unspecified),
            ffi::git_fetch_prune_t_GIT_FETCH_PRUNE => Ok(Self::Prune),
            ffi::git_fetch_prune_t_GIT_FETCH_NO_PRUNE => Ok(Self::NoPrune),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn remote_names_use_a_typed_optional_c_string() {
        // SAFETY: initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        assert_eq!(git_remote_name_is_valid(Some(c"origin")), Ok(true));
        assert_eq!(git_remote_name_is_valid(None), Ok(false));
        // SAFETY: balances this test's successful initialization.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn fetch_prune_has_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitFetchPrune>(),
            size_of::<ffi::git_fetch_prune_t>()
        );
        assert_eq!(
            align_of::<GitFetchPrune>(),
            align_of::<ffi::git_fetch_prune_t>()
        );
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::Unspecified), 0);
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::Prune), 1);
        assert_eq!(ffi::git_fetch_prune_t::from(GitFetchPrune::NoPrune), 2);
    }

    #[test]
    fn fetch_prune_validates_raw_values() {
        assert_eq!(GitFetchPrune::try_from(0), Ok(GitFetchPrune::Unspecified));
        assert_eq!(GitFetchPrune::try_from(2), Ok(GitFetchPrune::NoPrune));
        assert_eq!(GitFetchPrune::try_from(3), Err(3));
    }

    #[test]
    fn remote_autotag_options_have_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitRemoteAutotagOption>(),
            size_of::<ffi::git_remote_autotag_option_t>()
        );
        assert_eq!(
            align_of::<GitRemoteAutotagOption>(),
            align_of::<ffi::git_remote_autotag_option_t>()
        );

        for (option, raw) in [
            (GitRemoteAutotagOption::Unspecified, 0),
            (GitRemoteAutotagOption::Auto, 1),
            (GitRemoteAutotagOption::None, 2),
            (GitRemoteAutotagOption::All, 3),
        ] {
            assert_eq!(ffi::git_remote_autotag_option_t::from(option), raw);
            assert_eq!(GitRemoteAutotagOption::try_from(raw), Ok(option));
        }
        assert_eq!(GitRemoteAutotagOption::try_from(4), Err(4));
    }

    #[test]
    fn remote_completion_values_have_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitRemoteCompletion>(),
            size_of::<ffi::git_remote_completion_t>()
        );
        assert_eq!(
            align_of::<GitRemoteCompletion>(),
            align_of::<ffi::git_remote_completion_t>()
        );

        for (completion, raw) in [
            (GitRemoteCompletion::Download, 0),
            (GitRemoteCompletion::Indexing, 1),
            (GitRemoteCompletion::Error, 2),
        ] {
            assert_eq!(ffi::git_remote_completion_t::from(completion), raw);
            assert_eq!(GitRemoteCompletion::try_from(raw), Ok(completion));
        }
        assert_eq!(GitRemoteCompletion::try_from(3), Err(3));
    }

    #[test]
    fn remote_redirect_has_the_c_layout_and_values() {
        assert_eq!(
            size_of::<GitRemoteRedirect>(),
            size_of::<ffi::git_remote_redirect_t>()
        );
        assert_eq!(
            align_of::<GitRemoteRedirect>(),
            align_of::<ffi::git_remote_redirect_t>()
        );
        assert_eq!(ffi::git_remote_redirect_t::from(GitRemoteRedirect::None), 1);
        assert_eq!(
            ffi::git_remote_redirect_t::from(GitRemoteRedirect::Initial),
            2
        );
        assert_eq!(ffi::git_remote_redirect_t::from(GitRemoteRedirect::All), 4);
    }

    #[test]
    fn remote_redirect_defaults_to_initial_and_validates_raw_values() {
        assert_eq!(GitRemoteRedirect::default(), GitRemoteRedirect::Initial);
        assert_eq!(GitRemoteRedirect::try_from(1), Ok(GitRemoteRedirect::None));
        assert_eq!(GitRemoteRedirect::try_from(4), Ok(GitRemoteRedirect::All));
        assert_eq!(GitRemoteRedirect::try_from(0), Err(0));
        assert_eq!(GitRemoteRedirect::try_from(3), Err(3));
    }
}

/// Wraps: git_remote_autotag_option_t
/// Controls which tags are downloaded while fetching from a remote.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitRemoteAutotagOption {
    /// Use the remote or repository configuration.
    #[default]
    Unspecified = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_UNSPECIFIED,
    /// Download tags that point to objects already being downloaded.
    Auto = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_AUTO,
    /// Do not download tags beyond those selected by refspecs.
    None = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_NONE,
    /// Download every tag.
    All = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL,
}

impl From<GitRemoteAutotagOption> for ffi::git_remote_autotag_option_t {
    fn from(value: GitRemoteAutotagOption) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_remote_autotag_option_t> for GitRemoteAutotagOption {
    type Error = ffi::git_remote_autotag_option_t;

    fn try_from(value: ffi::git_remote_autotag_option_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_UNSPECIFIED => {
                Ok(Self::Unspecified)
            }
            ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_AUTO => Ok(Self::Auto),
            ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_NONE => Ok(Self::None),
            ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL => Ok(Self::All),
            other => Err(other),
        }
    }
}

/// Wraps: git_remote_completion_t
/// Identifies the remote operation reported by a completion callback.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitRemoteCompletion {
    /// Object downloading finished.
    Download = ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_DOWNLOAD,
    /// Indexing downloaded objects finished.
    Indexing = ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_INDEXING,
    /// The remote operation failed.
    Error = ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_ERROR,
}

impl From<GitRemoteCompletion> for ffi::git_remote_completion_t {
    fn from(value: GitRemoteCompletion) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_remote_completion_t> for GitRemoteCompletion {
    type Error = ffi::git_remote_completion_t;

    fn try_from(value: ffi::git_remote_completion_t) -> Result<Self, ffi::git_remote_completion_t> {
        match value {
            ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_DOWNLOAD => Ok(Self::Download),
            ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_INDEXING => Ok(Self::Indexing),
            ffi::git_remote_completion_t_GIT_REMOTE_COMPLETION_ERROR => Ok(Self::Error),
            other => Err(other),
        }
    }
}

/// Wraps: git_remote_redirect_t
/// Controls when a remote operation may follow an off-site redirect.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitRemoteRedirect {
    /// Never follow an off-site redirect.
    None = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_NONE,
    /// Follow an off-site redirect only for the initial request.
    #[default]
    Initial = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_INITIAL,
    /// Follow off-site redirects at any stage.
    All = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_ALL,
}

impl From<GitRemoteRedirect> for ffi::git_remote_redirect_t {
    fn from(value: GitRemoteRedirect) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_remote_redirect_t> for GitRemoteRedirect {
    type Error = ffi::git_remote_redirect_t;

    fn try_from(value: ffi::git_remote_redirect_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_NONE => Ok(Self::None),
            ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_INITIAL => Ok(Self::Initial),
            ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_ALL => Ok(Self::All),
            other => Err(other),
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_remote
    /// Opaque remote configuration and connection state managed by libgit2.
    ///
    /// Repository-backed remotes borrow their repository, which must remain
    /// alive while the remote is used. Detached remotes have no repository.
    /// Owned handles release the remote with `git_remote_free`; cloning makes
    /// an independent deep copy with `git_remote_dup`.
    GitRemote,
    GitRemoteRef,
    GitRemoteMut,
    ffi::git_remote
);

/// An owned libgit2 remote.
pub type GitRemoteOwned = CBox<GitRemote>;

// SAFETY: `git_remote_free` destroys a fully initialized remote and all of its
// owned fields. `GitRemote` is transparent over the corresponding bindgen type.
ffibox::impl_dropped!(GitRemote, ffi::git_remote, ffi::git_remote_free);

// SAFETY: `git_remote_dup` deep-copies the source's owned strings and refspecs
// into a fresh allocation. A successful result is independent of the source
// and is released by the `CDropped` implementation above.
unsafe impl CCloned for GitRemote {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the trait caller supplies a live remote, `duplicate` is a
        // valid output slot, and the wrapper is layout-compatible with the C
        // type. The routine only reads the source and initializes a new owner.
        let result = unsafe {
            ffi::git_remote_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_remote>(),
            )
        };

        (result == 0)
            .then(|| NonNull::new(duplicate.cast::<Self>()))
            .flatten()
    }
}

#[cfg(test)]
mod remote_type_tests {
    use core::mem::{MaybeUninit, align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CCloned, CDropped};

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
            // this guard. Remote owners are dropped before the guard.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}

        assert_cell::<GitRemote>();
        assert_eq!(size_of::<GitRemote>(), size_of::<ffi::git_remote>());
        assert_eq!(align_of::<GitRemote>(), align_of::<ffi::git_remote>());
        assert_eq!(
            size_of::<GitRemoteRef<'_>>(),
            size_of::<*const ffi::git_remote>()
        );
        assert_eq!(
            size_of::<GitRemoteMut<'_>>(),
            size_of::<*mut ffi::git_remote>()
        );
        assert_eq!(
            size_of::<Option<GitRemoteOwned>>(),
            size_of::<*mut ffi::git_remote>()
        );
    }

    #[test]
    fn remote_registers_deep_copy_lifecycle() {
        fn assert_lifecycle<T: CDropped + CCloned>() {}
        assert_lifecycle::<GitRemote>();
    }

    #[test]
    fn owned_remote_deep_copy_has_independent_storage() {
        let _init = Libgit2Init::acquire();
        let mut raw = ptr::null_mut();

        // SAFETY: libgit2 is initialized, `raw` is a writable output slot,
        // and the byte string is NUL-terminated and immutable for the call.
        assert_eq!(
            unsafe {
                ffi::git_remote_create_detached(
                    core::ptr::addr_of_mut!(raw),
                    c"https://example.invalid/repo".as_ptr(),
                )
            },
            0
        );
        // SAFETY: the successful constructor returned a fresh remote whose
        // matching destructor is registered on `GitRemote`.
        let remote = unsafe { GitRemoteOwned::from_raw(raw) }
            .expect("git_remote_create_detached succeeded with a null remote");

        let duplicate = remote.try_clone().expect("git_remote_dup failed");
        assert_ne!(remote.as_ptr(), duplicate.as_ptr());
    }

    #[test]
    fn borrowed_handles_preserve_the_remote_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_remote>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_remote>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitRemoteRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitRemoteMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_remote>>()) });
    }
}

/// Wraps: git_push_transfer_progress_cb
/// Safe callable surface for push upload progress.
pub trait GitPushTransferProgressCallback {
    /// Reports the object and byte counts. A nonzero result stops the push.
    fn call(&mut self, current: u32, total: u32, bytes: usize) -> i32;
}

impl<F> GitPushTransferProgressCallback for F
where
    F: FnMut(u32, u32, usize) -> i32,
{
    fn call(&mut self, current: u32, total: u32, bytes: usize) -> i32 {
        self(current, total, bytes)
    }
}

/// Wraps: git_push_update_reference_cb
/// Safe callable surface for per-reference push status.
pub trait GitPushUpdateReferenceCallback {
    /// Reports a remote reference update. `status == None` denotes success.
    fn call(&mut self, refname: &core::ffi::CStr, status: Option<&core::ffi::CStr>) -> i32;
}

impl<F> GitPushUpdateReferenceCallback for F
where
    F: FnMut(&core::ffi::CStr, Option<&core::ffi::CStr>) -> i32,
{
    fn call(&mut self, refname: &core::ffi::CStr, status: Option<&core::ffi::CStr>) -> i32 {
        self(refname, status)
    }
}

/// Wraps: git_remote_name_is_valid
/// Checks whether `name` is a valid remote name.
pub fn git_remote_name_is_valid(name: Option<&core::ffi::CStr>) -> Result<bool, i32> {
    let mut valid = 0;
    let name = name.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
    // SAFETY: `valid` is writable and `name` is null or a live C string.
    let status = unsafe { ffi::git_remote_name_is_valid(&mut valid, name) };
    if status == 0 {
        Ok(valid != 0)
    } else {
        Err(status)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_push_update
    /// An update lent by libgit2 to a push-negotiation callback.
    ///
    /// The enclosing push owns the update and its reference-name strings.
    /// Borrowed handles must not outlive the callback invocation that supplied
    /// the update.
    GitPushUpdate,
    GitPushUpdateRef,
    GitPushUpdateMut,
    ffi::git_push_update
);

impl<'a> GitPushUpdateRef<'a> {
    /// Wraps: git_push_update.src
    /// Borrows the current target of the source reference.
    #[must_use]
    pub fn src(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the inline initialized OID
        // without forming a reference, and the field lives for this update
        // handle's full borrow.
        let ptr = unsafe { addr_of!((*self.as_ptr()).src) }.cast_mut();
        // SAFETY: `ptr` addresses the live inline OID and inherits `'a` from
        // the enclosing update handle.
        unsafe { OidRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Wraps: git_push_update.dst
    /// Borrows the new target of the destination reference.
    #[must_use]
    pub fn dst(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the inline initialized OID
        // without forming a reference, and the field lives for this update
        // handle's full borrow.
        let ptr = unsafe { addr_of!((*self.as_ptr()).dst) }.cast_mut();
        // SAFETY: `ptr` addresses the live inline OID and inherits `'a` from
        // the enclosing update handle.
        unsafe { OidRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Wraps: git_push_update.dst_refname
    /// Borrows the destination reference name.
    #[must_use]
    pub fn dst_refname(&self) -> &'a core::ffi::CStr {
        // SAFETY: this shared handle permits a raw-place read of the pointer
        // field without forming a reference to C-visible update storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).dst_refname).read() };
        assert!(
            !ptr.is_null(),
            "a valid libgit2 push update has a destination reference name"
        );
        // SAFETY: libgit2 owns a live NUL-terminated string for the lifetime
        // of the update and the null check above establishes a valid start.
        unsafe { core::ffi::CStr::from_ptr(ptr) }
    }

    /// Wraps: git_push_update.src_refname
    /// Borrows the source reference name.
    #[must_use]
    pub fn src_refname(&self) -> &'a core::ffi::CStr {
        // SAFETY: this shared handle permits a raw-place read of the pointer
        // field without forming a reference to C-visible update storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).src_refname).read() };
        assert!(
            !ptr.is_null(),
            "a valid libgit2 push update has a source reference name"
        );
        // SAFETY: libgit2 owns a live NUL-terminated string for the lifetime
        // of the update and the null check above establishes a valid start.
        unsafe { core::ffi::CStr::from_ptr(ptr) }
    }
}

#[cfg(test)]
mod push_update_tests {
    use core::mem::{align_of, size_of};

    use ffibox::CCell;

    use super::*;

    #[test]
    fn push_update_layout_and_fields_match_the_c_value() {
        fn assert_cell<T: CCell>() {}

        let mut raw = ffi::git_push_update {
            src_refname: c"refs/heads/source".as_ptr().cast_mut(),
            dst_refname: c"refs/heads/destination".as_ptr().cast_mut(),
            src: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [0x11; 32],
            },
            dst: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA256 as u8,
                id: [0x22; 32],
            },
        };

        assert_cell::<GitPushUpdate>();
        assert_eq!(
            size_of::<GitPushUpdate>(),
            size_of::<ffi::git_push_update>()
        );
        assert_eq!(
            align_of::<GitPushUpdate>(),
            align_of::<ffi::git_push_update>()
        );

        // SAFETY: `raw` is a fully initialized update and remains live and
        // unchanged for the shared handle's lifetime.
        let update = unsafe { GitPushUpdateRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(update.src_refname(), c"refs/heads/source");
        assert_eq!(update.dst_refname(), c"refs/heads/destination");
        assert_eq!(update.src().raw_bytes().elem(0), Some(0x11));
        assert_eq!(update.dst().raw_bytes().elem(0), Some(0x22));
    }
}
