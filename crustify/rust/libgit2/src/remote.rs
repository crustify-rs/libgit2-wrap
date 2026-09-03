//! Safe wrappers for libgit2 remote APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of};

use ffibox::{CBox, CCloned, CVal};

use crate::api::buffer::GitBufMut;
use crate::api::proxy::GitProxyOptionsRef;
use crate::api::remote::{
    GitFetchOptionsRef, GitPushOptionsRef, GitRemoteCallbacksMut, GitRemoteCallbacksRef,
    GitRemoteConnectOptionsMut, GitRemoteConnectOptionsRef, GitRemoteCreateOptions,
    GitRemoteCreateOptionsRef, GitRemoteUpdateFlags,
};
use crate::ffi;
use crate::indexer::IndexerProgressRef;
use crate::oid::{InvalidOidType, OidRef, OidType};
use crate::refspec::GitRefspecRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
use crate::strarray::GitStrArray;
use crate::strarray::GitStrArrayRef;
use crate::util::net::Direction;
use crate::util::net::RemoteHeadRef;

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
mod unit_tests {
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
    fn remote_redirect_validates_raw_enumeration_values() {
        assert_eq!(GitRemoteRedirect::try_from(1), Ok(GitRemoteRedirect::None));
        assert_eq!(GitRemoteRedirect::try_from(4), Ok(GitRemoteRedirect::All));
        assert_eq!(GitRemoteRedirect::try_from(0), Err(0));
        assert_eq!(GitRemoteRedirect::try_from(3), Err(3));
    }

    #[test]
    fn an_unspecified_redirect_field_round_trips_as_none() {
        // `GIT_FETCH_OPTIONS_INIT` and its siblings leave the field zero, and
        // libgit2 reads that as "consult http.followRedirects" rather than as
        // one of the published values.
        assert_eq!(GitRemoteRedirect::UNSPECIFIED_FIELD, 0);
        assert_eq!(GitRemoteRedirect::from_field(0), Ok(None));
        assert_eq!(GitRemoteRedirect::to_field(None), 0);

        for value in [
            GitRemoteRedirect::None,
            GitRemoteRedirect::Initial,
            GitRemoteRedirect::All,
        ] {
            let raw = GitRemoteRedirect::to_field(Some(value));
            assert_ne!(raw, GitRemoteRedirect::UNSPECIFIED_FIELD);
            assert_eq!(GitRemoteRedirect::from_field(raw), Ok(Some(value)));
        }

        assert_eq!(GitRemoteRedirect::from_field(3), Err(3));
        assert_eq!(GitRemoteRedirect::UNCONFIGURED, GitRemoteRedirect::Initial);
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
///
/// The three variants are exactly the values the C enumeration publishes.
/// An options field additionally holds zero, which is what
/// `GIT_FETCH_OPTIONS_INIT`, `GIT_PUSH_OPTIONS_INIT` and
/// `GIT_REMOTE_CONNECT_OPTIONS_INIT` leave behind: libgit2 reads that as
/// *unspecified* and resolves it from `http.followRedirects`, falling back to
/// [`GitRemoteRedirect::UNCONFIGURED`] when the repository has no such
/// setting. A field therefore reads and writes as
/// `Option<GitRemoteRedirect>`, through [`GitRemoteRedirect::from_field`] and
/// [`GitRemoteRedirect::to_field`], with `None` for that unspecified state.
/// There is deliberately no `Default`: choosing one of the three published
/// values would suppress the configuration lookup a zero field selects.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum GitRemoteRedirect {
    /// Never follow an off-site redirect.
    None = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_NONE,
    /// Follow an off-site redirect only for the initial request.
    Initial = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_INITIAL,
    /// Follow off-site redirects at any stage.
    All = ffi::git_remote_redirect_t_GIT_REMOTE_REDIRECT_ALL,
}

impl GitRemoteRedirect {
    /// The behaviour libgit2 applies to an unspecified field when the
    /// repository configures no `http.followRedirects` value, and for an
    /// operation that has no repository to consult.
    pub const UNCONFIGURED: Self = Self::Initial;

    /// The unspecified value an options initializer leaves in the field.
    pub const UNSPECIFIED_FIELD: ffi::git_remote_redirect_t = 0;

    /// Reads a `follow_redirects` options field. `Ok(None)` denotes the
    /// unspecified value that selects the configured behaviour.
    pub fn from_field(
        value: ffi::git_remote_redirect_t,
    ) -> Result<Option<Self>, ffi::git_remote_redirect_t> {
        if value == Self::UNSPECIFIED_FIELD {
            Ok(Option::None)
        } else {
            Self::try_from(value).map(Some)
        }
    }

    /// Encodes a `follow_redirects` options field. `None` stores the
    /// unspecified value that selects the configured behaviour.
    #[must_use]
    pub fn to_field(value: Option<Self>) -> ffi::git_remote_redirect_t {
        value.map_or(Self::UNSPECIFIED_FIELD, ffi::git_remote_redirect_t::from)
    }
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
    /// alive while the remote is used; [`GitRemoteWithRepository`] carries that
    /// borrow. Detached remotes have no repository, so
    /// [`git_remote_create_detached`] hands back a bare [`GitRemoteOwned`].
    /// Owned handles release the remote with `git_remote_free`; cloning
    /// duplicates the remote's own storage with `git_remote_dup` while keeping
    /// the source's repository borrow.
    GitRemote,
    GitRemoteRef,
    GitRemoteMut,
    ffi::git_remote
);

/// Wraps: git_remote_free
/// An owned libgit2 remote that disconnects and frees itself on drop.
pub type GitRemoteOwned = CBox<GitRemote>;

// SAFETY: `git_remote_free` destroys a fully initialized remote and all of its
// owned fields. `GitRemote` is transparent over the corresponding bindgen type.
ffibox::impl_dropped!(GitRemote, ffi::git_remote, ffi::git_remote_free);

/// Wraps: git_remote_dup
// SAFETY: `git_remote_dup` deep-copies the source's owned storage — name, URL,
// push URL and the textual refspecs, which it re-parses into fresh vectors —
// into a separate allocation released by the `CDropped` implementation above.
// It copies no transport, push, local head, negotiation or transfer state, so
// the result starts disconnected and frees nothing the source still owns.
//
// It does copy `source->repo` verbatim. That pointer is a borrow libgit2 never
// owns or releases, so the duplicate is tethered to exactly the same
// repository as its source and is only as long-lived. `CCloned` is therefore
// bound to `CBox<GitRemote>`, which safe code reaches only for the detached
// remotes built by `git_remote_create_detached` (`repo` null); a
// repository-backed remote is cloned through
// `GitRemoteWithRepository::try_clone`, which preserves the borrow.
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
        let remote = git_remote_create_detached(c"https://example.invalid/repo")
            .expect("detached remote creation succeeds");

        let duplicate = remote.try_clone().expect("git_remote_dup failed");
        assert_ne!(remote.as_ptr(), duplicate.as_ptr());

        // `git_remote_dup` re-duplicates the URL rather than aliasing it, so
        // both owners may run `git_remote_free` over their own storage.
        let source_url = git_remote_url(remote.as_ref()).expect("a detached remote has a URL");
        let copied_url = git_remote_url(duplicate.as_ref()).expect("the copy keeps the URL");
        assert_eq!(source_url, copied_url);
        assert!(!core::ptr::eq(source_url.as_ptr(), copied_url.as_ptr()));
    }

    #[test]
    fn a_repository_backed_duplicate_keeps_the_repository_borrow() {
        let _init = Libgit2Init::acquire();
        // A detached remote carries a null `repo`, which is the same borrow
        // shape `git_remote_dup` copies for a repository-backed one; tethering
        // it here exercises the wrapper without a repository fixture.
        let tethered =
            repository_remote_result(git_remote_create_detached(c"https://example.invalid/repo"))
                .expect("detached remote creation succeeds");

        let duplicate = tethered.try_clone().expect("git_remote_dup failed");
        assert_ne!(tethered.as_ref().as_ptr(), duplicate.as_ref().as_ptr());
        assert_eq!(
            git_remote_url(duplicate.as_ref()),
            Some(c"https://example.invalid/repo")
        );
    }

    #[test]
    fn populated_error_output_is_already_an_raii_owner() {
        let _init = Libgit2Init::acquire();
        let remote = git_remote_create_detached(c"https://example.invalid/repo")
            .expect("detached remote creation succeeds");

        assert!(matches!(remote_result(-1, Some(remote)), Err(-1)));
    }

    #[test]
    fn detached_remote_safe_surface_exposes_borrowed_state() {
        let _init = Libgit2Init::acquire();
        let mut remote = git_remote_create_detached(c"https://example.invalid/repo")
            .expect("detached remote creation succeeds");

        let shared = remote.as_ref();
        assert_eq!(git_remote_name(shared), None);
        assert_eq!(
            git_remote_url(shared),
            Some(c"https://example.invalid/repo")
        );
        assert_eq!(git_remote_pushurl(shared), None);
        assert!(!git_remote_connected(shared));
        assert_eq!(git_remote_refspec_count(shared), 0);
        assert!(git_remote_get_refspec(shared, 0).is_none());
        assert_eq!(git_remote_stats(shared).received_objects(), 0);

        assert_eq!(git_remote_disconnect(&mut remote.as_mut()), Ok(()));
        assert_eq!(git_remote_stop(&mut remote.as_mut()), Ok(()));
        assert!(matches!(
            git_remote_oid_type(&mut remote.as_mut()),
            Err(GitRemoteOidTypeError::Libgit2(_))
        ));
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
    /// Field: git_push_update.src
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

    /// Field: git_push_update.dst
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

    /// Field: git_push_update.dst_refname
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

    /// Field: git_push_update.src_refname
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

/// An owned repository-backed remote that cannot outlive the repository it
/// borrows internally.
pub struct GitRemoteWithRepository<'repo> {
    remote: GitRemoteOwned,
    _repository: PhantomData<&'repo ()>,
}

impl GitRemoteWithRepository<'_> {
    /// Borrows the remote shared.
    #[must_use]
    pub fn as_ref(&self) -> GitRemoteRef<'_> {
        self.remote.as_ref()
    }

    /// Borrows the remote exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> GitRemoteMut<'_> {
        self.remote.as_mut()
    }

    /// Transfers this repository-backed remote to a C owner.
    ///
    /// # Safety
    /// The repository borrowed by this remote must remain live until C frees
    /// the returned remote pointer.
    pub(crate) unsafe fn into_raw(self) -> *mut ffi::git_remote {
        self.remote.into_raw()
    }

    /// Wraps: git_remote_dup
    /// Duplicates the remote, keeping its repository borrow.
    ///
    /// `git_remote_dup` copies the source's repository pointer, so the
    /// duplicate depends on the same repository and keeps the same `'repo`.
    /// Returns `None` when libgit2 could not allocate the copy.
    #[must_use]
    pub fn try_clone(&self) -> Option<Self> {
        Some(Self {
            remote: self.remote.try_clone()?,
            _repository: PhantomData,
        })
    }
}

fn remote_result(status: i32, remote: Option<GitRemoteOwned>) -> Result<GitRemoteOwned, i32> {
    if status == 0 {
        Ok(remote.expect("libgit2 succeeded without returning a remote"))
    } else {
        Err(status)
    }
}

fn repository_remote_result<'repo>(
    result: Result<GitRemoteOwned, i32>,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    result.map(|remote| GitRemoteWithRepository {
        remote,
        _repository: PhantomData,
    })
}

/// Wraps: git_remote_add_fetch
/// Adds a fetch refspec to a persisted remote configuration.
pub fn git_remote_add_fetch(
    repo: &mut GitRepositoryMut<'_>,
    remote: &CStr,
    refspec: &CStr,
) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and both strings are live
    // for the call; libgit2 retains none of these pointers.
    let status =
        unsafe { ffi::git_remote_add_fetch(repo.as_mut_ptr(), remote.as_ptr(), refspec.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_add_push
/// Adds a push refspec to a persisted remote configuration.
pub fn git_remote_add_push(
    repo: &mut GitRepositoryMut<'_>,
    remote: &CStr,
    refspec: &CStr,
) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and both strings are live
    // for the call; libgit2 retains none of these pointers.
    let status =
        unsafe { ffi::git_remote_add_push(repo.as_mut_ptr(), remote.as_ptr(), refspec.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_connected
/// Reports whether the underlying transport is connected.
#[must_use]
pub fn git_remote_connected(remote: GitRemoteRef<'_>) -> bool {
    // SAFETY: `remote` is live and shared and the query retains no pointer.
    unsafe { ffi::git_remote_connected(remote.as_ptr()) != 0 }
}

/// Wraps: git_remote_create
/// Creates and persists a named remote that borrows `repo` for its lifetime.
pub fn git_remote_create<'repo>(
    repo: &'repo mut GitRepositoryMut<'_>,
    name: &CStr,
    url: &CStr,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the repository is live and remains
    // borrowed by the result, and both strings are live for the call.
    let status =
        unsafe { ffi::git_remote_create(&mut raw, repo.as_mut_ptr(), name.as_ptr(), url.as_ptr()) };
    // SAFETY: `raw` is null or the complete remote allocation transferred by
    // the output slot. Turning it into RAII immediately also cleans up an
    // unexpected populated error output.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    repository_remote_result(remote_result(status, remote))
}

/// Wraps: git_remote_create_anonymous
/// Creates an in-memory remote that borrows `repo` for its lifetime.
pub fn git_remote_create_anonymous<'repo>(
    repo: &'repo mut GitRepositoryMut<'_>,
    url: &CStr,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the repository remains borrowed by
    // the result, and `url` is a live transient C string.
    let status =
        unsafe { ffi::git_remote_create_anonymous(&mut raw, repo.as_mut_ptr(), url.as_ptr()) };
    // SAFETY: `raw` is null or the complete remote allocation transferred by
    // the output slot. The owner is lifetime-tethered below before it escapes.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    repository_remote_result(remote_result(status, remote))
}

/// Wraps: git_remote_create_detached
/// Creates an independently owned remote with no local repository.
pub fn git_remote_create_detached(url: &CStr) -> Result<GitRemoteOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and `url` is a live transient C
    // string. Detached remotes retain no repository pointer.
    let status = unsafe { ffi::git_remote_create_detached(&mut raw, url.as_ptr()) };
    // SAFETY: `raw` is null or the complete detached remote allocation
    // transferred through the output slot.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    remote_result(status, remote)
}

/// Wraps: git_remote_create_with_fetchspec
/// Creates a named remote with an optional fetch refspec and a repository
/// borrow that lasts as long as the result.
pub fn git_remote_create_with_fetchspec<'repo>(
    repo: &'repo mut GitRepositoryMut<'_>,
    name: &CStr,
    url: &CStr,
    fetch: Option<&CStr>,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    let fetch = fetch.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the output slot is writable, the repository remains borrowed by
    // the result, required strings are live, and `fetch` is null or live.
    let status = unsafe {
        ffi::git_remote_create_with_fetchspec(
            &mut raw,
            repo.as_mut_ptr(),
            name.as_ptr(),
            url.as_ptr(),
            fetch,
        )
    };
    // SAFETY: `raw` is null or the complete remote allocation transferred by
    // the output slot. The owner is lifetime-tethered below before it escapes.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    repository_remote_result(remote_result(status, remote))
}

/// Wraps: git_remote_default_branch
/// Writes the connected remote's default reference name into `out`.
pub fn git_remote_default_branch(
    out: &mut GitBufMut<'_>,
    remote: &mut GitRemoteMut<'_>,
) -> Result<(), i32> {
    // SAFETY: both handles are live and exclusive for the operation and the
    // output buffer retains its own resulting allocation.
    let status = unsafe { ffi::git_remote_default_branch(out.as_mut_ptr(), remote.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_delete
/// Deletes a persisted remote and its remote-tracking configuration.
pub fn git_remote_delete(repo: &mut GitRepositoryMut<'_>, name: &CStr) -> Result<(), i32> {
    // SAFETY: the repository is live and exclusive and `name` is a live
    // transient C string.
    let status = unsafe { ffi::git_remote_delete(repo.as_mut_ptr(), name.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_disconnect
/// Closes the remote's transport connection when one is open.
pub fn git_remote_disconnect(remote: &mut GitRemoteMut<'_>) -> Result<(), i32> {
    // SAFETY: `remote` is live and exclusively borrowed for transport mutation.
    let status = unsafe { ffi::git_remote_disconnect(remote.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

fn remote_refspecs(
    remote: GitRemoteRef<'_>,
    get: unsafe extern "C" fn(*mut ffi::git_strarray, *const ffi::git_remote) -> i32,
) -> Result<CVal<GitStrArray>, i32> {
    let mut array = GitStrArray::new();
    let status = {
        let mut out = array.as_mut();
        // SAFETY: `out` is an empty exclusive output header and `remote` is a
        // live shared input. The selected getter initializes owned strings on
        // success and leaves the header disposable on failure.
        unsafe { get(out.as_mut_ptr(), remote.as_ptr()) }
    };
    if status == 0 { Ok(array) } else { Err(status) }
}

/// Wraps: git_remote_get_fetch_refspecs
/// Deep-copies all configured fetch refspec strings.
pub fn git_remote_get_fetch_refspecs(remote: GitRemoteRef<'_>) -> Result<CVal<GitStrArray>, i32> {
    remote_refspecs(remote, ffi::git_remote_get_fetch_refspecs)
}

/// Wraps: git_remote_get_push_refspecs
/// Deep-copies all configured push refspec strings.
pub fn git_remote_get_push_refspecs(remote: GitRemoteRef<'_>) -> Result<CVal<GitStrArray>, i32> {
    remote_refspecs(remote, ffi::git_remote_get_push_refspecs)
}

/// Wraps: git_remote_get_refspec
/// Borrows the refspec at `index`, if present.
#[must_use]
pub fn git_remote_get_refspec<'a>(
    remote: GitRemoteRef<'a>,
    index: usize,
) -> Option<GitRefspecRef<'a>> {
    // SAFETY: `remote` is live and shared; a non-null result is owned by its
    // refspec vector and remains live for the remote borrow.
    let spec = unsafe { ffi::git_remote_get_refspec(remote.as_ptr(), index) }.cast_mut();
    // SAFETY: the non-null case is the borrowed refspec described above.
    unsafe { GitRefspecRef::from_ptr(spec) }
}

/// Wraps: git_remote_list
/// Returns an owned list of configured remote names.
pub fn git_remote_list(repo: &mut GitRepositoryMut<'_>) -> Result<CVal<GitStrArray>, i32> {
    let mut list = GitStrArray::new();
    let status = {
        let mut out = list.as_mut();
        // SAFETY: `out` is an empty exclusive header and `repo` is live and
        // exclusive. Success installs newly owned strings in `out`.
        unsafe { ffi::git_remote_list(out.as_mut_ptr(), repo.as_mut_ptr()) }
    };
    if status == 0 { Ok(list) } else { Err(status) }
}

/// Wraps: git_remote_lookup
/// Loads a configured remote that borrows `repo` for its lifetime.
pub fn git_remote_lookup<'repo>(
    repo: &'repo mut GitRepositoryMut<'_>,
    name: &CStr,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable, the repository remains borrowed by
    // the result, and `name` is a live transient C string.
    let status = unsafe { ffi::git_remote_lookup(&mut raw, repo.as_mut_ptr(), name.as_ptr()) };
    // SAFETY: `raw` is null or the complete remote allocation transferred by
    // the output slot. The owner is lifetime-tethered below before it escapes.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    repository_remote_result(remote_result(status, remote))
}

/// Wraps: git_remote_name
/// Borrows the optional persisted name of a remote.
#[must_use]
pub fn git_remote_name<'a>(remote: GitRemoteRef<'a>) -> Option<&'a CStr> {
    // SAFETY: returns null or a remote-owned NUL string live for this borrow.
    let name = unsafe { ffi::git_remote_name(remote.as_ptr()) };
    if name.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL string described above.
        Some(unsafe { CStr::from_ptr(name) })
    }
}

/// Error returned while querying a connected remote's object-ID algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitRemoteOidTypeError {
    /// Libgit2 rejected the query.
    Libgit2(i32),
    /// Libgit2 returned an unknown algorithm value.
    Invalid(InvalidOidType),
}

/// Wraps: git_remote_oid_type
/// Returns the connected remote's object-ID algorithm.
pub fn git_remote_oid_type(
    remote: &mut GitRemoteMut<'_>,
) -> Result<OidType, GitRemoteOidTypeError> {
    let mut raw = 0;
    // SAFETY: `raw` is writable and `remote` is live and exclusive for the
    // transport query.
    let status = unsafe { ffi::git_remote_oid_type(&mut raw, remote.as_mut_ptr()) };
    if status != 0 {
        return Err(GitRemoteOidTypeError::Libgit2(status));
    }
    OidType::try_from(raw).map_err(GitRemoteOidTypeError::Invalid)
}

/// Wraps: git_remote_pushurl
/// Borrows the optional special push URL.
#[must_use]
pub fn git_remote_pushurl<'a>(remote: GitRemoteRef<'a>) -> Option<&'a CStr> {
    // SAFETY: returns null or a remote-owned NUL string live for this borrow.
    let url = unsafe { ffi::git_remote_pushurl(remote.as_ptr()) };
    if url.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL string described above.
        Some(unsafe { CStr::from_ptr(url) })
    }
}

/// Wraps: git_remote_refspec_count
/// Returns the number of configured refspecs.
#[must_use]
pub fn git_remote_refspec_count(remote: GitRemoteRef<'_>) -> usize {
    // SAFETY: `remote` is live and shared and the getter retains no pointer.
    unsafe { ffi::git_remote_refspec_count(remote.as_ptr()) }
}

/// Wraps: git_remote_rename
/// Renames a persisted remote and returns refspecs that could not be updated.
pub fn git_remote_rename(
    repo: &mut GitRepositoryMut<'_>,
    name: &CStr,
    new_name: &CStr,
) -> Result<CVal<GitStrArray>, i32> {
    let mut problems = GitStrArray::new();
    let status = {
        let mut out = problems.as_mut();
        // SAFETY: `out` is an empty exclusive header, `repo` is live and
        // exclusive, and both strings are live for the call.
        unsafe {
            ffi::git_remote_rename(
                out.as_mut_ptr(),
                repo.as_mut_ptr(),
                name.as_ptr(),
                new_name.as_ptr(),
            )
        }
    };
    if status == 0 {
        Ok(problems)
    } else {
        Err(status)
    }
}

/// Wraps: git_remote_set_pushurl
/// Sets, or with `None` deletes, the configured push URL.
pub fn git_remote_set_pushurl(
    repo: &mut GitRepositoryMut<'_>,
    remote: &CStr,
    url: Option<&CStr>,
) -> Result<(), i32> {
    let url = url.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the repository is live and exclusive, `remote` is live, and
    // `url` is null or a live transient C string.
    let status = unsafe { ffi::git_remote_set_pushurl(repo.as_mut_ptr(), remote.as_ptr(), url) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_set_url
/// Sets, or with `None` deletes, the configured fetch URL.
pub fn git_remote_set_url(
    repo: &mut GitRepositoryMut<'_>,
    remote: &CStr,
    url: Option<&CStr>,
) -> Result<(), i32> {
    let url = url.map_or(core::ptr::null(), CStr::as_ptr);
    // SAFETY: the repository is live and exclusive, `remote` is live, and
    // `url` is null or a live transient C string.
    let status = unsafe { ffi::git_remote_set_url(repo.as_mut_ptr(), remote.as_ptr(), url) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_stats
/// Borrows the remote's inline transfer statistics.
#[must_use]
pub fn git_remote_stats<'a>(remote: GitRemoteRef<'a>) -> IndexerProgressRef<'a> {
    // SAFETY: source inspection shows this legacy mutable parameter is read
    // only; the returned pointer addresses a non-null inline field.
    let stats = unsafe { ffi::git_remote_stats(remote.as_ptr().cast_mut()) }.cast_mut();
    // SAFETY: a live remote always contains this inline progress record.
    unsafe { IndexerProgressRef::from_ptr(stats) }
        .expect("a remote contains inline transfer statistics")
}

/// Wraps: git_remote_stop
/// Requests cancellation of the current network operation.
pub fn git_remote_stop(remote: &mut GitRemoteMut<'_>) -> Result<(), i32> {
    // SAFETY: `remote` is live and exclusively borrowed for cancellation.
    let status = unsafe { ffi::git_remote_stop(remote.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_url
/// Borrows the remote's optional fetch URL.
#[must_use]
pub fn git_remote_url<'a>(remote: GitRemoteRef<'a>) -> Option<&'a CStr> {
    // SAFETY: returns null or a remote-owned NUL string live for this borrow.
    let url = unsafe { ffi::git_remote_url(remote.as_ptr()) };
    if url.is_null() {
        None
    } else {
        // SAFETY: non-null is the live NUL string described above.
        Some(unsafe { CStr::from_ptr(url) })
    }
}

/// A borrowed array of remote-advertised head pointers.
#[derive(Clone, Copy)]
pub struct GitRemoteHeads<'a> {
    ptr: NonNull<*const ffi::git_remote_head>,
    len: usize,
    _remote: PhantomData<GitRemoteRef<'a>>,
}

impl<'a> GitRemoteHeads<'a> {
    /// Returns the number of advertised heads.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether the remote advertised no heads.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one advertised head by index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<RemoteHeadRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: `git_remote_ls` supplied a readable array of `len` head
        // pointers and the bounds check selects one initialized slot.
        let head = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: a successful list contains live non-null heads owned by the
        // remote for the duration of `'a`.
        unsafe { RemoteHeadRef::from_ptr(head.cast_mut()) }
    }
}

/// Wraps: git_remote_ls
/// Borrows the advertised heads from a connected remote.
pub fn git_remote_ls<'a>(remote: &'a mut GitRemoteMut<'_>) -> Result<GitRemoteHeads<'a>, i32> {
    let mut heads: *mut *const ffi::git_remote_head = core::ptr::null_mut();
    let mut len = 0;
    // SAFETY: both output slots are writable and the remote is exclusively
    // borrowed, preventing disconnect while the returned array is observed.
    let status = unsafe {
        ffi::git_remote_ls(
            core::ptr::addr_of_mut!(heads),
            core::ptr::addr_of_mut!(len),
            remote.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    let ptr = if len == 0 && heads.is_null() {
        NonNull::dangling()
    } else {
        NonNull::new(heads.cast_mut()).ok_or(ffi::git_error_code_GIT_ERROR)?
    };
    Ok(GitRemoteHeads {
        ptr,
        len,
        _remote: PhantomData,
    })
}

/// Wraps: git_remote_connect
/// Connects a remote using callback and proxy state valid until disconnect.
///
/// The callback and proxy payloads are `'static` because the transport keeps
/// them: `git_smart__connect` calls `git_remote_connect_options_normalize`,
/// whose `git_remote_connect_options_dup` copies the callback table and the
/// proxy options into `t->connect_opts`, and the transport then reaches
/// `certificate_check` and `credentials` with that stored payload on every
/// later request until `git_remote_disconnect` or `git_remote_free` releases
/// it. The headers need no such bound: the same `dup` runs
/// `git_strarray_copy` over them.
pub fn git_remote_connect(
    remote: &mut GitRemoteMut<'_>,
    direction: Direction,
    callbacks: Option<GitRemoteCallbacksRef<'_, 'static>>,
    proxy: Option<GitProxyOptionsRef<'_, 'static>>,
    custom_headers: Option<GitStrArrayRef<'_>>,
) -> Result<(), i32> {
    // SAFETY: `remote` is exclusive; the headers are copied synchronously, and
    // callback/proxy nested borrows are restricted to `'static` because the
    // connected transport may use them until a later disconnect.
    let status = unsafe {
        ffi::git_remote_connect(
            remote.as_mut_ptr(),
            direction.into(),
            callbacks.map_or(core::ptr::null(), |callbacks| callbacks.as_ptr()),
            proxy.map_or(core::ptr::null(), |proxy| proxy.as_ptr()),
            custom_headers.map_or(core::ptr::null(), |headers| headers.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_connect_ext
/// Connects a remote with a complete connection-options record.
///
/// Nested callback and proxy payloads must be `'static` because transport
/// normalization copies them into transport state used by later requests, and
/// the connection this call opens outlives the exclusive remote borrow.
/// Header strings and the proxy URL are deep-copied by libgit2.
///
/// The bound is load-bearing rather than a restatement of an unsafe setter's
/// contract, exactly as it is on [`git_remote_download`]:
/// [`GitProxyOptionsMut::set_url`](crate::api::proxy::GitProxyOptionsMut::set_url)
/// and
/// [`GitProxyOptionsMut::set_credentials`](crate::api::proxy::GitProxyOptionsMut::set_credentials)
/// install nested data from safe code, so nested data that does not live for
/// `'static` is rejected:
///
/// ```compile_fail,E0597
/// use core::ffi::CStr;
///
/// use libgit2::api::remote::GitRemoteConnectOptions;
/// use libgit2::remote::{GitRemoteMut, git_remote_connect_ext};
/// use libgit2::util::net::Direction;
///
/// fn connect(remote: &mut GitRemoteMut<'_>, proxy: &CStr) {
///     let proxy = proxy.to_owned();
///     let mut options = GitRemoteConnectOptions::new();
///     options.as_mut().proxy_options_mut().set_url(Some(&proxy));
///     let _ = git_remote_connect_ext(remote, Direction::Fetch, Some(options.as_ref()));
/// }
/// ```
pub fn git_remote_connect_ext(
    remote: &mut GitRemoteMut<'_>,
    direction: Direction,
    options: Option<GitRemoteConnectOptionsRef<'_, 'static>>,
) -> Result<(), i32> {
    // SAFETY: `remote` is exclusive; the options header stays live for the
    // synchronous call, and all nested referents that the transport may retain
    // are restricted to `'static` by the safe signature.
    let status = unsafe {
        ffi::git_remote_connect_ext(
            remote.as_mut_ptr(),
            direction.into(),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_connect_options_init
/// Initializes remote-connection options for `version`.
pub fn git_remote_connect_options_init(
    options: &mut GitRemoteConnectOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer retains no pointer into it.
    let status = unsafe { ffi::git_remote_connect_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_init_callbacks
/// Initializes a remote callback table for `version`.
pub fn git_remote_init_callbacks(
    callbacks: &mut GitRemoteCallbacksMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable callback-table storage;
    // initialization retains no pointer into it.
    let status = unsafe { ffi::git_remote_init_callbacks(callbacks.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_prune
/// Removes remote-tracking references no longer advertised by the remote.
///
/// Unlike [`git_remote_connect`], nothing here survives the call: the callback
/// table's `update_refs` entry and payload are read as each stale reference is
/// deleted and no copy is stored, so the callbacks need only outlive the call.
pub fn git_remote_prune(
    remote: &mut GitRemoteMut<'_>,
    callbacks: Option<GitRemoteCallbacksRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the remote is exclusive and callbacks, when present, stay live
    // for the synchronous prune operation.
    let status = unsafe {
        ffi::git_remote_prune(
            remote.as_mut_ptr(),
            callbacks.map_or(core::ptr::null(), |callbacks| callbacks.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_update_tips
/// Updates remote-tracking tips from the connected remote's advertisements.
///
/// As in [`git_remote_prune`], the callback table is read by
/// `update_tips_for_spec` and `opportunistic_updates` while each tip is
/// written and never copied, so it need only outlive the call.
pub fn git_remote_update_tips(
    remote: &mut GitRemoteMut<'_>,
    callbacks: Option<GitRemoteCallbacksRef<'_, '_>>,
    update_flags: GitRemoteUpdateFlags,
    download_tags: GitRemoteAutotagOption,
    reflog_message: Option<&CStr>,
) -> Result<(), i32> {
    // SAFETY: the remote is exclusive and every optional borrowed input stays
    // live for the synchronous update operation.
    let status = unsafe {
        ffi::git_remote_update_tips(
            remote.as_mut_ptr(),
            callbacks.map_or(core::ptr::null(), |callbacks| callbacks.as_ptr()),
            update_flags.bits(),
            download_tags.into(),
            reflog_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_connection_tests {
    use super::*;
    use crate::api::remote::GitRemoteCallbacks;

    #[test]
    fn callback_initializer_writes_the_current_version() {
        let mut callbacks = GitRemoteCallbacks::new();
        git_remote_init_callbacks(&mut callbacks.as_mut(), ffi::GIT_REMOTE_CALLBACKS_VERSION)
            .unwrap();
        assert_eq!(
            callbacks.as_ref().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
    }

    #[test]
    fn the_connect_options_initializer_restores_every_constructed_default() {
        // The C initializer copies `GIT_REMOTE_CONNECT_OPTIONS_INIT` over the
        // whole record, so reinitializing deliberately dirtied options pins
        // `GitRemoteConnectOptions::new` to that template. Only the two
        // nested headers carry a nonzero default; the redirect policy stays
        // unspecified so that configuration is consulted.
        let mut options = crate::api::remote::GitRemoteConnectOptions::new();
        {
            let mut view = options.as_mut();
            view.set_version(0);
            view.set_follow_redirects(Some(GitRemoteRedirect::All));
            view.callbacks_mut().set_version(0);
            view.proxy_options_mut().set_version(0);
            view.proxy_options_mut()
                .set_proxy_type(crate::proxy::ProxyType::Auto);
        }

        git_remote_connect_options_init(
            &mut options.as_mut(),
            ffi::GIT_REMOTE_CONNECT_OPTIONS_VERSION,
        )
        .unwrap();

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_REMOTE_CONNECT_OPTIONS_VERSION);
        assert_eq!(
            view.callbacks().version(),
            ffi::GIT_REMOTE_CALLBACKS_VERSION
        );
        assert_eq!(
            view.proxy_options().version(),
            ffi::GIT_PROXY_OPTIONS_VERSION
        );
        assert_eq!(
            view.proxy_options().proxy_type(),
            Ok(crate::proxy::ProxyType::None)
        );
        assert_eq!(view.proxy_options().url(), None);
        assert_eq!(view.follow_redirects(), Ok(None));
        assert_eq!(view.custom_headers().count(), 0);
    }

    #[test]
    fn an_unsupported_connect_options_version_is_rejected_without_writing() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced below; the rejected version reaches `git_error_set`, which
        // allocates through the allocator only initialization installs.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut options = crate::api::remote::GitRemoteConnectOptions::new();
        options
            .as_mut()
            .set_follow_redirects(Some(GitRemoteRedirect::All));
        assert!(git_remote_connect_options_init(&mut options.as_mut(), 0).is_err());
        assert_eq!(
            options.as_ref().follow_redirects(),
            Ok(Some(GitRemoteRedirect::All))
        );
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn extended_connect_requires_static_nested_callback_data() {
        let _: fn(
            &mut GitRemoteMut<'_>,
            Direction,
            Option<GitRemoteConnectOptionsRef<'_, 'static>>,
        ) -> Result<(), i32> = git_remote_connect_ext;
    }
}

/// Wraps: git_remote_download
/// Downloads and indexes the objects selected by `refspecs`, leaving the
/// remote connected.
///
/// The remote must be repository-backed; a detached one is rejected with
/// `GIT_ERROR_INVALID` before anything else runs. Refspec strings are parsed
/// into the remote's own refspec vectors during the call and never retained.
///
/// The options' nested data is `'static` because this call installs it in a
/// connection it does not close. `git_remote_connect_options_dup` deep-copies
/// the custom headers with `git_strarray_copy` and the proxy URL with
/// `git__strdup`, but plain-copies the callback and proxy payload slots along
/// with their function pointers, and those copies live in the transport's own
/// connect options, which every later request reads back through
/// `git_smart__credentials`, `git_smart__certificate_check` and
/// `handle_proxy_auth`. The bound is load-bearing rather than a restatement of
/// the unsafe [`GitRemoteCallbacksMut::set_handler`] contract, because
/// [`GitProxyOptionsMut::set_credentials`] installs the proxy payload from
/// safe code. Nothing on the way out clears the copies either:
/// `git_smart__close` disposes the stream and not the options, so they outlive
/// a later [`git_remote_disconnect`] and are replaced only by the next
/// connect or reconfigure, or released when the remote is freed.
///
/// [`GitProxyOptionsMut::set_credentials`]: crate::api::proxy::GitProxyOptionsMut::set_credentials
///
/// Nested data that does not live for `'static` is therefore rejected:
///
/// ```compile_fail
/// use core::ffi::CStr;
///
/// use libgit2::api::remote::GitFetchOptions;
/// use libgit2::remote::{GitRemoteMut, git_remote_download};
///
/// fn download(remote: &mut GitRemoteMut<'_>, proxy: &CStr) {
///     let proxy = proxy.to_owned();
///     let mut options = GitFetchOptions::new();
///     options.as_mut().proxy_options_mut().set_url(Some(&proxy));
///     let _ = git_remote_download(remote, None, Some(options.as_ref()));
/// }
/// ```
pub fn git_remote_download(
    remote: &mut GitRemoteMut<'_>,
    refspecs: Option<GitStrArrayRef<'_>>,
    options: Option<GitFetchOptionsRef<'_, 'static>>,
) -> Result<(), i32> {
    // SAFETY: the remote is live and exclusive, the refspec header and its
    // strings stay live for the call, and the options' nested payload slots
    // are static because the connection this call leaves open keeps copies of
    // them.
    let status = unsafe {
        ffi::git_remote_download(
            remote.as_mut_ptr(),
            refspecs.map_or(core::ptr::null(), |values| values.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_fetch
/// Downloads objects, disconnects, updates remote-tracking tips and prunes as
/// requested.
///
/// The options carry the `'static` nested data that [`git_remote_download`]
/// documents, and the closing disconnect does not relax it.
/// `connect_or_reset_options` installs the copies first, and the
/// `git_remote_capabilities` and `git_remote_oid_type` queries that follow
/// return early on failure without reaching `git_remote_disconnect`, leaving
/// the remote connected over the copies.
/// Even the ordinary path only closes the stream, so the copied payload slots
/// stay in the transport exactly as they do after a download.
///
/// `reflog_message` is copied into a `git_str` for the tip updates; `None`
/// selects libgit2's `fetch <name-or-url>` default.
pub fn git_remote_fetch(
    remote: &mut GitRemoteMut<'_>,
    refspecs: Option<GitStrArrayRef<'_>>,
    options: Option<GitFetchOptionsRef<'_, 'static>>,
    reflog_message: Option<&CStr>,
) -> Result<(), i32> {
    // SAFETY: the remote is live and exclusive, the refspec header and reflog
    // string are transient live borrows, and the options' nested payload slots
    // are static because an early error can leave the copied connection state
    // installed.
    let status = unsafe {
        ffi::git_remote_fetch(
            remote.as_mut_ptr(),
            refspecs.map_or(core::ptr::null(), |values| values.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
            reflog_message.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_push
/// Uploads the selected refspecs, updates remote-tracking tips and disconnects.
///
/// Unlike [`git_remote_download`], the options' nested data need only outlive
/// the call, and the unconditional `git_remote_disconnect` on the way out is
/// what makes the difference: every path that reached `git_remote_upload`, and
/// so every path that could have installed a copy in the transport, runs it.
/// The copies libgit2 does keep past the call are unreachable rather than
/// refreshed. The transport's connect options survive the close, but each of
/// their readers is behind a `git_remote_connect_options_normalize` that
/// replaces them first; and `git_push_new` copies the callback table into
/// `remote->push`, where only `do_push` reads it, inside this call's
/// `git_push_finish`. A later [`git_remote_update_tips`] does reach that same
/// `git_push`, but hands `git_push_update_tips` its own callbacks.
///
/// The remote is disconnected even when the caller connected it beforehand.
pub fn git_remote_push(
    remote: &mut GitRemoteMut<'_>,
    refspecs: Option<GitStrArrayRef<'_>>,
    options: Option<GitPushOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    // SAFETY: the remote is live and exclusive and all optional inputs remain
    // live for the synchronous push, which disconnects before returning and
    // leaves no copy of them that another wrapped operation can reach.
    let status = unsafe {
        ffi::git_remote_push(
            remote.as_mut_ptr(),
            refspecs.map_or(core::ptr::null(), |values| values.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_transfer_tests {
    use super::*;
    use crate::api::errors::GitErrorClass;
    use crate::api::remote::{GitFetchOptions, GitPushOptions};
    use crate::util::errors::git_error_last;

    #[test]
    fn typed_transfer_inputs_reach_detached_remote_validation() {
        // SAFETY: libgit2 initialization is refcounted and balanced after all
        // owners constructed by this test have been dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        let mut remote = git_remote_create_detached(c"https://example.invalid/repo")
            .expect("detached remote creation succeeds");
        let refspecs = GitStrArray::new();
        let fetch_options = GitFetchOptions::new();
        let push_options = GitPushOptions::new();

        // All four reject a repository-less remote before reading the
        // options, so each records the same `GIT_ERROR_INVALID` refusal.
        fn assert_detached_refusal(status: Result<(), i32>) {
            assert_eq!(status, Err(-1));
            let error = git_error_last();
            assert_eq!(error.klass, Ok(GitErrorClass::Invalid));
            assert_eq!(
                error.message.as_deref(),
                Some(c"cannot download detached remote")
            );
        }

        assert_detached_refusal(git_remote_download(
            &mut remote.as_mut(),
            Some(refspecs.as_ref()),
            Some(fetch_options.as_ref()),
        ));
        assert_detached_refusal(git_remote_fetch(
            &mut remote.as_mut(),
            Some(refspecs.as_ref()),
            Some(fetch_options.as_ref()),
            Some(c"fetch test"),
        ));
        assert_detached_refusal(git_remote_upload(
            &mut remote.as_mut(),
            Some(refspecs.as_ref()),
            Some(push_options.as_ref()),
        ));
        assert_detached_refusal(git_remote_push(
            &mut remote.as_mut(),
            Some(refspecs.as_ref()),
            Some(push_options.as_ref()),
        ));

        drop(push_options);
        drop(fetch_options);
        drop(refspecs);
        drop(remote);
        // SAFETY: balances this test's successful initialization after every
        // libgit2-backed owner has been released.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

#[cfg(test)]
mod scheduled_transfer_repository_tests {
    use super::*;
    use crate::api::errors::GitErrorClass;
    use crate::api::remote::{GitFetchOptions, GitPushOptions};
    use crate::repository::git_repository_open;
    use crate::util::errors::git_error_last;

    /// A refcounted hold on the process-global libgit2 initialization.
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
            // this guard; every owner opened under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built bare repository, plus a sibling path that holds no
    /// repository at all for the transports to fail against.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-remote-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects/info")).expect("a loose-object directory");
            std::fs::create_dir_all(path.join("objects/pack")).expect("a pack directory");
            std::fs::create_dir_all(path.join("refs/heads")).expect("a refs directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            Self::to_c(self.0.to_str().expect("a UTF-8 temporary path"))
        }

        /// A `file://` URL naming a directory this fixture never creates, so
        /// the local transport fails in `git_repository_open`.
        fn absent_url(&self) -> std::ffi::CString {
            let path = self.0.to_str().expect("a UTF-8 temporary path");
            Self::to_c(&format!("file://{path}-absent"))
        }

        /// A `file://` URL naming the fixture itself, so the local transport
        /// connects.
        fn url(&self) -> std::ffi::CString {
            let path = self.0.to_str().expect("a UTF-8 temporary path");
            Self::to_c(&format!("file://{path}"))
        }

        fn to_c(value: &str) -> std::ffi::CString {
            std::ffi::CString::new(value).expect("a path without interior NUL")
        }
    }

    impl Drop for BareRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn transfers_leave_a_repository_backed_remote_unconnected_when_the_url_is_absent() {
        let _init = Libgit2Init::acquire();
        let directory = BareRepo::create("absent-url");
        let mut repository =
            git_repository_open(&directory.c_path()).expect("the bare repository opens");
        let mut owner = repository.as_mut();
        let mut remote = git_remote_create_anonymous(&mut owner, &directory.absent_url())
            .expect("an anonymous remote for a local URL");

        // Past the detached-remote guard, each transfer reaches the transport
        // and fails opening the missing repository, leaving nothing connected.
        let fetch_options = GitFetchOptions::new();
        let push_options = GitPushOptions::new();

        assert_eq!(
            git_remote_download(&mut remote.as_mut(), None, Some(fetch_options.as_ref())),
            Err(-1)
        );
        assert!(!git_remote_connected(remote.as_ref()));

        assert_eq!(
            git_remote_fetch(
                &mut remote.as_mut(),
                None,
                Some(fetch_options.as_ref()),
                Some(c"fetch test"),
            ),
            Err(-1)
        );
        assert!(!git_remote_connected(remote.as_ref()));

        assert_eq!(
            git_remote_push(&mut remote.as_mut(), None, Some(push_options.as_ref())),
            Err(-1)
        );
        assert!(!git_remote_connected(remote.as_ref()));
    }

    #[test]
    fn push_options_may_borrow_data_released_before_the_remote() {
        let _init = Libgit2Init::acquire();
        let directory = BareRepo::create("borrowed-push-data");
        let mut repository =
            git_repository_open(&directory.c_path()).expect("the bare repository opens");
        let mut owner = repository.as_mut();
        // An unroutable scheme stops the push in `git_transport_new`, after
        // the options have been read but before any transport exists to copy
        // them into.
        let mut remote =
            git_remote_create_anonymous(&mut owner, c"crustify://example.invalid/repo")
                .expect("an anonymous remote for an unsupported URL");

        {
            // Every borrowed input below is created after the remote and
            // released before it, which type-checks only because
            // `git_remote_push` does not require `'static` nested data.
            let header = std::ffi::CString::new("X-Crustify: review").expect("a valid header");
            let mut entries = [header.as_ptr().cast_mut()];
            let mut headers = ffi::git_strarray {
                strings: entries.as_mut_ptr(),
                count: entries.len(),
            };
            // SAFETY: `headers` is an initialized header over a live pointer
            // run whose single entry is a live NUL-terminated string, and both
            // outlive the handle borrowed here.
            let headers = unsafe { GitStrArrayRef::from_ptr(&raw mut headers) }
                .expect("the address of a stack value is non-null");
            let proxy = std::ffi::CString::new("http://proxy.invalid/").expect("a valid URL");

            let mut push_options = GitPushOptions::new();
            {
                let mut view = push_options.as_mut();
                view.set_custom_headers(headers);
                view.proxy_options_mut().set_url(Some(&proxy));
            }

            // `git_remote_connect_options_normalize` validates and deep-copies
            // both borrowed inputs during the call; the push then stops at the
            // unsupported scheme.
            assert_eq!(
                git_remote_push(&mut remote.as_mut(), None, Some(push_options.as_ref())),
                Err(-1)
            );
            let error = git_error_last();
            assert_eq!(error.klass, Ok(GitErrorClass::Net));
            assert_eq!(error.message.as_deref(), Some(c"unsupported URL protocol"));
        }

        // A malformed header is rejected by that same normalization, before
        // any transport work, proving the borrowed strings are read rather
        // than merely stored.
        let malformed = std::ffi::CString::new("malformed").expect("a valid string");
        let mut entries = [malformed.as_ptr().cast_mut()];
        let mut headers = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: as above, the header and its single entry outlive the handle.
        let headers = unsafe { GitStrArrayRef::from_ptr(&raw mut headers) }
            .expect("the address of a stack value is non-null");
        let mut push_options = GitPushOptions::new();
        push_options.as_mut().set_custom_headers(headers);
        assert_eq!(
            git_remote_push(&mut remote.as_mut(), None, Some(push_options.as_ref())),
            Err(-1)
        );
        let error = git_error_last();
        assert_eq!(error.klass, Ok(GitErrorClass::Invalid));
        assert_eq!(
            error.message.as_deref(),
            Some(c"custom HTTP header 'malformed' is malformed")
        );
        assert!(!git_remote_connected(remote.as_ref()));
    }

    /// The connection [`git_remote_upload`] opens is what forces `'static`
    /// nested option data, and it is still open when the call returns — the
    /// distinction from [`git_remote_push`], which disconnects unconditionally.
    #[test]
    fn upload_leaves_the_transport_connected_after_a_refspec_failure() {
        let _init = Libgit2Init::acquire();
        let directory = BareRepo::create("upload-stays-connected");
        let mut repository =
            git_repository_open(&directory.c_path()).expect("the bare repository opens");
        let mut owner = repository.as_mut();
        let mut remote = git_remote_create_anonymous(&mut owner, &directory.url())
            .expect("an anonymous remote for the fixture itself");

        // `git_push_add_refspec` runs after `connect_or_reset_options`, and
        // `check_lref` rejects a source that names no object in the empty
        // fixture, so the failure lands with the transport already connected.
        let refspec = c"refs/heads/absent:refs/heads/absent";
        let mut entries = [refspec.as_ptr().cast_mut()];
        let mut header = ffi::git_strarray {
            strings: entries.as_mut_ptr(),
            count: entries.len(),
        };
        // SAFETY: `header` is an initialized string-array header over a live
        // pointer run whose single entry is a live NUL-terminated string, and
        // both outlive the handle borrowed here.
        let refspecs = unsafe { GitStrArrayRef::from_ptr(&raw mut header) }
            .expect("the address of a stack value is non-null");

        // Default options keep every retained slot null, so the transport copy
        // this call installs borrows nothing at all.
        let push_options = GitPushOptions::new();
        assert_eq!(
            git_remote_upload(
                &mut remote.as_mut(),
                Some(refspecs),
                Some(push_options.as_ref()),
            ),
            Err(-1)
        );
        let error = git_error_last();
        assert_eq!(error.klass, Ok(GitErrorClass::Reference));
        assert_eq!(
            error.message.as_deref(),
            Some(c"src refspec 'refs/heads/absent' does not match any existing object")
        );

        assert!(git_remote_connected(remote.as_ref()));
        assert_eq!(git_remote_disconnect(&mut remote.as_mut()), Ok(()));
        assert!(!git_remote_connected(remote.as_ref()));
    }
}

/// Wraps: git_remote_autotag
/// Returns the remote's tag-download policy.
pub fn git_remote_autotag(
    remote: GitRemoteRef<'_>,
) -> Result<GitRemoteAutotagOption, ffi::git_remote_autotag_option_t> {
    // SAFETY: `remote` is a live shared input and the getter retains nothing.
    GitRemoteAutotagOption::try_from(unsafe { ffi::git_remote_autotag(remote.as_ptr()) })
}

/// Wraps: git_remote_owner
/// Borrows the repository associated with a remote, if any.
#[must_use]
pub fn git_remote_owner<'a>(remote: GitRemoteRef<'a>) -> Option<GitRepositoryRef<'a>> {
    // SAFETY: the remote is live and C returns null or its borrowed repository.
    let repository = unsafe { ffi::git_remote_owner(remote.as_ptr()) };
    // SAFETY: a non-null repository remains live for the remote borrow.
    unsafe { GitRepositoryRef::from_ptr(repository) }
}

/// Wraps: git_remote_prune_refs
/// Reports whether fetches through this remote prune stale references.
#[must_use]
pub fn git_remote_prune_refs(remote: GitRemoteRef<'_>) -> bool {
    // SAFETY: `remote` is live and shared and the getter retains nothing.
    unsafe { ffi::git_remote_prune_refs(remote.as_ptr()) != 0 }
}

/// Wraps: git_remote_set_autotag
/// Stores the tag-download policy for a named repository remote.
pub fn git_remote_set_autotag(
    repository: &mut GitRepositoryMut<'_>,
    remote_name: &CStr,
    option: GitRemoteAutotagOption,
) -> Result<(), i32> {
    // SAFETY: the repository is exclusive, the name is a live C string, and
    // the checked enum is a published C value. No input pointer is retained.
    let status = unsafe {
        ffi::git_remote_set_autotag(repository.as_mut_ptr(), remote_name.as_ptr(), option.into())
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_set_instance_pushurl
/// Replaces this remote instance's push URL with a copied string.
pub fn git_remote_set_instance_pushurl(
    remote: &mut GitRemoteMut<'_>,
    url: &CStr,
) -> Result<(), i32> {
    // SAFETY: the remote is exclusive and `url` is live; C duplicates the
    // string before returning and retains no borrowed pointer.
    let status = unsafe { ffi::git_remote_set_instance_pushurl(remote.as_mut_ptr(), url.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_set_instance_url
/// Replaces this remote instance's fetch URL with a copied string.
pub fn git_remote_set_instance_url(remote: &mut GitRemoteMut<'_>, url: &CStr) -> Result<(), i32> {
    // SAFETY: the remote is exclusive and `url` is live; C duplicates the
    // string before returning and retains no borrowed pointer.
    let status = unsafe { ffi::git_remote_set_instance_url(remote.as_mut_ptr(), url.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_remote_create_options_init
/// Initializes remote-creation options for the requested ABI version.
pub fn git_remote_create_options_init(
    version: core::ffi::c_uint,
) -> Result<CVal<GitRemoteCreateOptions>, i32> {
    let mut options = GitRemoteCreateOptions::new();
    let status = {
        let mut output = options.as_mut();
        // SAFETY: `output` is writable layout-compatible storage and C does
        // not retain its address.
        unsafe { ffi::git_remote_create_options_init(output.as_mut_ptr(), version) }
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

/// Wraps: git_remote_create_with_opts
/// Creates a remote from borrowing options and keeps the result tied to them.
///
/// `'repo` is the options borrow, not the stored repository's: the options
/// wrapper carries no `'data` parameter, so a repository installed with
/// [`crate::api::remote::GitRemoteCreateOptionsMut::set_borrowed_repository`]
/// is kept alive by that setter's contract, which already covers every remote
/// created from the options. Creation writes through the stored pointer and
/// the new remote records it, so the borrow that setter demands is exclusive.
pub fn git_remote_create_with_opts<'repo>(
    url: &CStr,
    options: GitRemoteCreateOptionsRef<'repo>,
) -> Result<GitRemoteWithRepository<'repo>, i32> {
    repository_remote_result(remote_create_with_opts_raw(url, Some(options)))
}

/// Wraps: git_remote_create_with_opts
/// Creates the detached variant selected by a null options pointer.
pub fn git_remote_create_with_default_options(url: &CStr) -> Result<GitRemoteOwned, i32> {
    remote_create_with_opts_raw(url, None)
}

fn remote_create_with_opts_raw(
    url: &CStr,
    options: Option<GitRemoteCreateOptionsRef<'_>>,
) -> Result<GitRemoteOwned, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output is writable, `url` is live, and `options` is null or
    // live. Its stored borrowed values satisfy their accessor-setter contract.
    let status = unsafe {
        ffi::git_remote_create_with_opts(
            core::ptr::addr_of_mut!(raw),
            url.as_ptr(),
            options.map_or(core::ptr::null(), |value| value.as_ptr()),
        )
    };
    // SAFETY: `raw` is null or one complete transferred remote allocation.
    let remote = unsafe { GitRemoteOwned::from_raw(raw) };
    remote_result(status, remote)
}

/// Wraps: git_remote_upload
/// Uploads the selected refspecs and leaves the remote connected.
///
/// Every string input is copied during the call: `git_refspec__parse`
/// duplicates each refspec, `git_remote_connect_options_dup` runs
/// `git_strarray_copy` over the custom headers and `git__strdup` over the
/// proxy URL, and this function `git__strdup`s each remote push option into
/// the push state.
///
/// The payload slots are not copied, only their pointers, and this operation
/// deliberately returns without disconnecting.
/// `connect_or_reset_options` installs the callback table and the proxy
/// options — payloads included — in the transport's own `connect_opts`, where
/// `git_transport_smart_credentials`, `git_transport_smart_certificate_check`
/// and the sideband and transfer-progress readers reach them on every later
/// request over the connection this call leaves open. Nothing on the way out
/// clears them either: `git_smart__close` disposes the stream and not the
/// options, so they survive a later [`git_remote_disconnect`], are replaced
/// only by the next connect or reconfigure, and are released only with the
/// remote. Nested option data must therefore be `'static`.
///
/// `git_push_new` copies the callback table into `remote->push` as well, and
/// that copy also outlives the call, but only `do_push` reads it and that runs
/// inside this call's `git_push_finish`. A later [`git_remote_update_tips`]
/// does reach the same `git_push`, and hands `git_push_update_tips` its own
/// callbacks. The transport copy is what makes the bound load-bearing.
///
/// The bound is not a restatement of the unsafe
/// [`GitRemoteCallbacksMut::set_handler`] contract, because
/// [`GitProxyOptionsMut::set_credentials`] installs the retained proxy payload
/// from safe code. `'data` is one parameter for the whole options tree, so it
/// also pins the deep-copied URL that the rejected snippet below installs:
///
/// [`GitRemoteCallbacksMut::set_handler`]: crate::api::remote::GitRemoteCallbacksMut::set_handler
/// [`GitProxyOptionsMut::set_credentials`]: crate::api::proxy::GitProxyOptionsMut::set_credentials
///
/// ```compile_fail,E0597
/// use core::ffi::CStr;
///
/// use libgit2::api::remote::GitPushOptions;
/// use libgit2::remote::{GitRemoteMut, git_remote_upload};
///
/// fn upload(remote: &mut GitRemoteMut<'_>, proxy: &CStr) {
///     let proxy = proxy.to_owned();
///     let mut options = GitPushOptions::new();
///     options.as_mut().proxy_options_mut().set_url(Some(&proxy));
///     let _ = git_remote_upload(remote, None, Some(options.as_ref()));
/// }
/// ```
pub fn git_remote_upload(
    remote: &mut GitRemoteMut<'_>,
    refspecs: Option<GitStrArrayRef<'_>>,
    options: Option<GitPushOptionsRef<'_, 'static>>,
) -> Result<(), i32> {
    // SAFETY: the remote is live and exclusive, refspecs remain live until
    // their strings have been copied, and any nested payload pointer copied
    // into the retained transport or push state is valid for the program.
    let status = unsafe {
        ffi::git_remote_upload(
            remote.as_mut_ptr(),
            refspecs.map_or(core::ptr::null(), |values| values.as_ptr()),
            options.map_or(core::ptr::null(), |options| options.as_ptr()),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{
        HistoryFixture, Libgit2Init, RawRepository, TempDir, safe_buf_bytes,
    };

    fn file_url(path: &std::path::Path) -> std::ffi::CString {
        std::ffi::CString::new(format!("file://{}", path.to_str().unwrap())).unwrap()
    }

    unsafe fn raw_send(remote: *mut ffi::git_remote, spec: &CStr, upload: bool) {
        let mut value = spec.as_ptr().cast_mut();
        let refspecs = ffi::git_strarray {
            strings: &mut value,
            count: 1,
        };
        let status = if upload {
            unsafe { ffi::git_remote_upload(remote, &refspecs, core::ptr::null()) }
        } else {
            unsafe { ffi::git_remote_push(remote, &refspecs, core::ptr::null()) }
        };
        assert_eq!(status, 0);
    }

    fn safe_send(remote: &mut GitRemoteMut<'_>, spec: &CStr, upload: bool) {
        let mut value = spec.as_ptr().cast_mut();
        let mut raw_refspecs = ffi::git_strarray {
            strings: &mut value,
            count: 1,
        };
        let refspecs = unsafe { GitStrArrayRef::from_ptr(&raw mut raw_refspecs) }.unwrap();
        if upload {
            git_remote_upload(remote, Some(refspecs), None).unwrap();
        } else {
            git_remote_push(remote, Some(refspecs), None).unwrap();
        }
    }

    unsafe fn raw_push(source: *mut ffi::git_repository, destination: &core::ffi::CStr) {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create(&mut remote, source, c"sink".as_ptr(), destination.as_ptr())
            },
            0
        );
        unsafe { raw_send(remote, c"refs/heads/master:refs/heads/copied", false) };
        unsafe { raw_send(remote, c":refs/heads/copied", false) };
        unsafe { raw_send(remote, c"+refs/heads/master:refs/heads/copied", true) };
        assert_eq!(
            unsafe {
                ffi::git_remote_update_tips(
                    remote,
                    core::ptr::null(),
                    0,
                    ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_UNSPECIFIED,
                    c"upload update".as_ptr(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_remote_disconnect(remote) }, 0);
        unsafe { ffi::git_remote_free(remote) };
    }

    fn safe_push(
        source: &mut crate::repository::GitRepositoryMut<'_>,
        destination: &core::ffi::CStr,
    ) {
        let mut remote = git_remote_create(source, c"sink", destination).unwrap();
        safe_send(
            &mut remote.as_mut(),
            c"refs/heads/master:refs/heads/copied",
            false,
        );
        safe_send(&mut remote.as_mut(), c":refs/heads/copied", false);
        safe_send(
            &mut remote.as_mut(),
            c"+refs/heads/master:refs/heads/copied",
            true,
        );
        git_remote_update_tips(
            &mut remote.as_mut(),
            None,
            GitRemoteUpdateFlags::EMPTY,
            GitRemoteAutotagOption::Unspecified,
            Some(c"upload update"),
        )
        .unwrap();
        git_remote_disconnect(&mut remote.as_mut()).unwrap();
    }

    unsafe fn destination_observation(repository: *mut ffi::git_repository) -> (Vec<u8>, usize) {
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut id, repository, c"refs/heads/copied".as_ptr())
            },
            0
        );
        let mut odb = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_repository_odb(&mut odb, repository) }, 0);
        unsafe extern "C" fn count(
            _id: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { *payload.cast::<usize>() += 1 };
            0
        }
        let mut objects = 0usize;
        assert_eq!(
            unsafe {
                ffi::git_odb_foreach(odb, Some(count), core::ptr::from_mut(&mut objects).cast())
            },
            0
        );
        unsafe { ffi::git_odb_free(odb) };
        (id.id.to_vec(), objects)
    }

    #[test]
    fn io_equiv_local_transport_push() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_source = HistoryFixture::new("push-source-raw");
        let safe_source = HistoryFixture::new("push-source-safe");
        let raw_directory = TempDir::new("push-target-raw");
        let safe_directory = TempDir::new("push-target-safe");
        let raw_path = raw_directory.c_path();
        let safe_path = safe_directory.c_path();
        let raw_target = RawRepository::init(&raw_path, true).unwrap();
        let safe_target = RawRepository::init(&safe_path, true).unwrap();
        let raw_url = file_url(raw_directory.path());
        let safe_url = file_url(safe_directory.path());

        unsafe { raw_push(raw_source.repository.as_ptr(), &raw_url) };
        let mut safe_repository = unsafe {
            crate::repository::GitRepositoryMut::from_ptr(safe_source.repository.as_ptr())
        }
        .unwrap();
        safe_push(&mut safe_repository, &safe_url);

        let raw_observation = unsafe { destination_observation(raw_target.as_ptr()) };
        let safe_observation = unsafe { destination_observation(safe_target.as_ptr()) };
        assert_eq!(raw_observation, safe_observation);
        assert!(raw_observation.1 >= 10);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct FetchObservation {
        remotes: usize,
        rename_status: i32,
        fetch_refspecs: usize,
        push_refspecs: usize,
        refspec_count: usize,
        name: Vec<u8>,
        url: Vec<u8>,
        pushurl: Vec<u8>,
        default_branch: Vec<u8>,
        advertised_heads: usize,
        received_objects: u32,
        fetched_id: Vec<u8>,
        connected_after_disconnect: bool,
        tracking_ref_renamed: bool,
    }

    unsafe fn raw_fetch_management(
        repository: *mut ffi::git_repository,
        url: &core::ffi::CStr,
    ) -> FetchObservation {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create_with_fetchspec(
                    &mut remote,
                    repository,
                    c"origin".as_ptr(),
                    url.as_ptr(),
                    c"+refs/heads/*:refs/remotes/origin/*".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        assert_eq!(
            unsafe {
                ffi::git_remote_add_fetch(
                    repository,
                    c"origin".as_ptr(),
                    c"+refs/tags/*:refs/tags/*".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_add_push(
                    repository,
                    c"origin".as_ptr(),
                    c"refs/heads/master:refs/heads/mirror".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_remote_set_pushurl(repository, c"origin".as_ptr(), url.as_ptr(),) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_set_autotag(
                    repository,
                    c"origin".as_ptr(),
                    ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL,
                )
            },
            0
        );
        remote = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_lookup(&mut remote, repository, c"origin".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_fetch(
                    remote,
                    core::ptr::null(),
                    core::ptr::null(),
                    c"pre-rename fetch".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        remote = core::ptr::null_mut();
        let mut problems = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        let rename_status = unsafe {
            ffi::git_remote_rename(
                &mut problems,
                repository,
                c"origin".as_ptr(),
                c"upstream".as_ptr(),
            )
        };
        unsafe { ffi::git_strarray_dispose(&mut problems) };
        let mut tracking = core::ptr::null_mut();
        let tracking_ref_renamed = unsafe {
            ffi::git_reference_lookup(
                &mut tracking,
                repository,
                c"refs/remotes/upstream/master".as_ptr(),
            ) == 0
        };
        if !tracking.is_null() {
            unsafe { ffi::git_reference_free(tracking) };
        }
        let mut remotes = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        assert_eq!(unsafe { ffi::git_remote_list(&mut remotes, repository) }, 0);
        let remote_count = remotes.count;
        unsafe { ffi::git_strarray_dispose(&mut remotes) };

        assert_eq!(
            unsafe { ffi::git_remote_lookup(&mut remote, repository, c"upstream".as_ptr()) },
            0
        );
        let mut fetch = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        let mut push = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        assert_eq!(
            unsafe { ffi::git_remote_get_fetch_refspecs(&mut fetch, remote) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_remote_get_push_refspecs(&mut push, remote) },
            0
        );
        let fetch_refspecs = fetch.count;
        let push_refspecs = push.count;
        unsafe {
            ffi::git_strarray_dispose(&mut fetch);
            ffi::git_strarray_dispose(&mut push);
        }
        assert_eq!(
            unsafe {
                ffi::git_remote_fetch(
                    remote,
                    core::ptr::null(),
                    core::ptr::null(),
                    c"equivalence fetch".as_ptr(),
                )
            },
            0
        );
        let stats = unsafe { ffi::git_remote_stats(remote) };
        let received_objects = unsafe { (*stats).received_objects };
        assert_eq!(
            unsafe {
                ffi::git_remote_connect(
                    remote,
                    ffi::git_direction_GIT_DIRECTION_FETCH,
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut heads = core::ptr::null_mut();
        let mut advertised_heads = 0;
        assert_eq!(
            unsafe { ffi::git_remote_ls(&mut heads, &mut advertised_heads, remote) },
            0
        );
        let mut default_branch = unsafe { core::mem::zeroed::<ffi::git_buf>() };
        assert_eq!(
            unsafe { ffi::git_remote_default_branch(&mut default_branch, remote) },
            0
        );
        let default_branch_bytes = unsafe {
            core::slice::from_raw_parts(default_branch.ptr.cast::<u8>(), default_branch.size)
                .to_vec()
        };
        unsafe { ffi::git_buf_dispose(&mut default_branch) };
        assert_eq!(unsafe { ffi::git_remote_disconnect(remote) }, 0);
        let connected_after_disconnect = unsafe { ffi::git_remote_connected(remote) != 0 };
        let name = unsafe { core::ffi::CStr::from_ptr(ffi::git_remote_name(remote)) }
            .to_bytes()
            .to_vec();
        let remote_url = unsafe { core::ffi::CStr::from_ptr(ffi::git_remote_url(remote)) }
            .to_bytes()
            .to_vec();
        let pushurl = unsafe { core::ffi::CStr::from_ptr(ffi::git_remote_pushurl(remote)) }
            .to_bytes()
            .to_vec();
        let refspec_count = unsafe { ffi::git_remote_refspec_count(remote) };
        assert!(!unsafe { ffi::git_remote_get_refspec(remote, 0) }.is_null());
        let mut fetched = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut fetched,
                    repository,
                    c"refs/remotes/upstream/master".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        FetchObservation {
            remotes: remote_count,
            rename_status,
            fetch_refspecs,
            push_refspecs,
            refspec_count,
            name,
            url: remote_url,
            pushurl,
            default_branch: default_branch_bytes,
            advertised_heads,
            received_objects,
            fetched_id: fetched.id.to_vec(),
            connected_after_disconnect,
            tracking_ref_renamed,
        }
    }

    fn safe_fetch_management(
        repository: *mut ffi::git_repository,
        url: &core::ffi::CStr,
    ) -> FetchObservation {
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        drop(
            git_remote_create_with_fetchspec(
                &mut repository_view,
                c"origin",
                url,
                Some(c"+refs/heads/*:refs/remotes/origin/*"),
            )
            .unwrap(),
        );
        git_remote_add_fetch(&mut repository_view, c"origin", c"+refs/tags/*:refs/tags/*").unwrap();
        git_remote_add_push(
            &mut repository_view,
            c"origin",
            c"refs/heads/master:refs/heads/mirror",
        )
        .unwrap();
        git_remote_set_pushurl(&mut repository_view, c"origin", Some(url)).unwrap();
        git_remote_set_autotag(&mut repository_view, c"origin", GitRemoteAutotagOption::All)
            .unwrap();
        let mut before_rename = git_remote_lookup(&mut repository_view, c"origin").unwrap();
        git_remote_fetch(
            &mut before_rename.as_mut(),
            None,
            None,
            Some(c"pre-rename fetch"),
        )
        .unwrap();
        drop(before_rename);
        let rename_status = match git_remote_rename(&mut repository_view, c"origin", c"upstream") {
            Ok(_) => 0,
            Err(error) => error,
        };
        let mut tracking = core::ptr::null_mut();
        let tracking_ref_renamed = unsafe {
            ffi::git_reference_lookup(
                &mut tracking,
                repository,
                c"refs/remotes/upstream/master".as_ptr(),
            ) == 0
        };
        if !tracking.is_null() {
            unsafe { ffi::git_reference_free(tracking) };
        }
        let remotes = git_remote_list(&mut repository_view)
            .unwrap()
            .as_ref()
            .count();
        let mut remote = git_remote_lookup(&mut repository_view, c"upstream").unwrap();
        let fetch_refspecs = git_remote_get_fetch_refspecs(remote.as_ref())
            .unwrap()
            .as_ref()
            .count();
        let push_refspecs = git_remote_get_push_refspecs(remote.as_ref())
            .unwrap()
            .as_ref()
            .count();
        git_remote_fetch(&mut remote.as_mut(), None, None, Some(c"equivalence fetch")).unwrap();
        let received_objects = git_remote_stats(remote.as_ref()).received_objects();
        git_remote_connect(
            &mut remote.as_mut(),
            crate::util::net::Direction::Fetch,
            None,
            None,
            None,
        )
        .unwrap();
        let advertised_heads = git_remote_ls(&mut remote.as_mut()).unwrap().len();
        let mut default_branch = crate::api::buffer::GitBuf::new();
        git_remote_default_branch(&mut default_branch.as_mut(), &mut remote.as_mut()).unwrap();
        let default_branch = safe_buf_bytes(default_branch.as_ref());
        git_remote_disconnect(&mut remote.as_mut()).unwrap();
        let connected_after_disconnect = git_remote_connected(remote.as_ref());
        let name = git_remote_name(remote.as_ref())
            .unwrap()
            .to_bytes()
            .to_vec();
        let remote_url = git_remote_url(remote.as_ref()).unwrap().to_bytes().to_vec();
        let pushurl = git_remote_pushurl(remote.as_ref())
            .unwrap()
            .to_bytes()
            .to_vec();
        let refspec_count = git_remote_refspec_count(remote.as_ref());
        assert!(git_remote_get_refspec(remote.as_ref(), 0).is_some());
        let mut fetched = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut fetched,
                    repository,
                    c"refs/remotes/upstream/master".as_ptr(),
                )
            },
            0
        );
        FetchObservation {
            remotes,
            rename_status,
            fetch_refspecs,
            push_refspecs,
            refspec_count,
            name,
            url: remote_url,
            pushurl,
            default_branch,
            advertised_heads,
            received_objects,
            fetched_id: fetched.id.to_vec(),
            connected_after_disconnect,
            tracking_ref_renamed,
        }
    }

    #[test]
    fn io_equiv_remote_configuration_fetch_and_advertisement_lifecycle() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("remote-fetch-source");
        let raw_directory = TempDir::new("remote-fetch-raw");
        let safe_directory = TempDir::new("remote-fetch-safe");
        let raw_path = raw_directory.c_path();
        let safe_path = safe_directory.c_path();
        let raw_target = RawRepository::init(&raw_path, true).unwrap();
        let safe_target = RawRepository::init(&safe_path, true).unwrap();
        let url = file_url(source.directory.path());
        let raw = unsafe { raw_fetch_management(raw_target.as_ptr(), &url) };
        assert_eq!(raw, safe_fetch_management(safe_target.as_ptr(), &url));
        assert_eq!(raw.remotes, 1);
        assert!(!raw.connected_after_disconnect);
        assert!(raw.advertised_heads >= 2);
    }

    struct SmartHttpServer {
        url: std::ffi::CString,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for SmartHttpServer {
        fn drop(&mut self) {
            self.stop.store(true, std::sync::atomic::Ordering::Release);
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
        }
    }

    fn decode_chunked_request(bytes: &[u8]) -> Option<Vec<u8>> {
        let mut position = 0;
        let mut decoded = Vec::new();
        loop {
            let line_end = bytes[position..]
                .windows(2)
                .position(|window| window == b"\r\n")?
                + position;
            let size = std::str::from_utf8(&bytes[position..line_end])
                .ok()?
                .split(';')
                .next()
                .and_then(|value| usize::from_str_radix(value.trim(), 16).ok())?;
            position = line_end + 2;
            if size == 0 {
                return Some(decoded);
            }
            if bytes.len() < position + size + 2 {
                return None;
            }
            decoded.extend_from_slice(&bytes[position..position + size]);
            position += size;
            if &bytes[position..position + 2] != b"\r\n" {
                return None;
            }
            position += 2;
        }
    }

    fn smart_http_server(source: &HistoryFixture) -> SmartHttpServer {
        smart_http_server_framed(source, false)
    }

    fn smart_http_server_framed(source: &HistoryFixture, chunked: bool) -> SmartHttpServer {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicBool, Ordering};

        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let project_root = source.directory.path().parent().unwrap().to_owned();
        let project = source.directory.path().file_name().unwrap().to_owned();
        let url = std::ffi::CString::new(format!(
            "http://127.0.0.1:{port}/{}",
            project.to_str().unwrap()
        ))
        .unwrap();
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            while !thread_stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("smart HTTP accept failed: {error}"),
                };
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 8192];
                let header_end = loop {
                    let read = stream.read(&mut buffer).unwrap();
                    assert_ne!(read, 0, "HTTP request ended before its headers");
                    request.extend_from_slice(&buffer[..read]);
                    if let Some(position) = request.windows(4).position(|part| part == b"\r\n\r\n")
                    {
                        break position + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let mut lines = headers.lines();
                let request_line = lines.next().unwrap();
                let mut request_parts = request_line.split_whitespace();
                let method = request_parts.next().unwrap().to_owned();
                let target = request_parts.next().unwrap().to_owned();
                let declared_content_length = lines
                    .clone()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .or_else(|| line.strip_prefix("content-length: "))
                    })
                    .map_or(0, |value| value.trim().parse::<usize>().unwrap());
                let content_type = lines
                    .find_map(|line| {
                        line.strip_prefix("Content-Type: ")
                            .or_else(|| line.strip_prefix("content-type: "))
                    })
                    .map(str::trim)
                    .unwrap_or("")
                    .to_owned();
                let request_is_chunked = headers
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("Transfer-Encoding: chunked"));
                let request_body = if request_is_chunked {
                    loop {
                        if let Some(decoded) = decode_chunked_request(&request[header_end..]) {
                            break decoded;
                        }
                        let read = stream.read(&mut buffer).unwrap();
                        assert_ne!(read, 0, "chunked HTTP request body ended early");
                        request.extend_from_slice(&buffer[..read]);
                    }
                } else {
                    while request.len() < header_end + declared_content_length {
                        let read = stream.read(&mut buffer).unwrap();
                        assert_ne!(read, 0, "HTTP request body ended early");
                        request.extend_from_slice(&buffer[..read]);
                    }
                    request[header_end..header_end + declared_content_length].to_vec()
                };
                let content_length = request_body.len();
                let (path, query) = target
                    .split_once('?')
                    .map_or((target.as_str(), ""), |(path, query)| (path, query));
                let mut child = std::process::Command::new("git")
                    .arg("http-backend")
                    .env("GIT_PROJECT_ROOT", &project_root)
                    .env("GIT_HTTP_EXPORT_ALL", "1")
                    .env("PATH_INFO", path)
                    .env("QUERY_STRING", query)
                    .env("REQUEST_METHOD", &method)
                    .env("CONTENT_TYPE", &content_type)
                    .env("CONTENT_LENGTH", content_length.to_string())
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();
                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(&request_body)
                    .unwrap();
                let output = child.wait_with_output().unwrap();
                assert!(
                    output.status.success(),
                    "git http-backend failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let split = output
                    .stdout
                    .windows(4)
                    .position(|part| part == b"\r\n\r\n")
                    .expect("CGI response separates headers from its body");
                let body = &output.stdout[split + 4..];
                stream.write_all(b"HTTP/1.1 200 OK\r\n").unwrap();
                stream.write_all(&output.stdout[..split]).unwrap();
                if chunked {
                    stream
                        .write_all(b"\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n")
                        .unwrap();
                    for chunk in body.chunks(97) {
                        write!(stream, "{:x}\r\n", chunk.len()).unwrap();
                        stream.write_all(chunk).unwrap();
                        stream.write_all(b"\r\n").unwrap();
                    }
                    stream
                        .write_all(b"0\r\nX-Equivalence: complete\r\n\r\n")
                        .unwrap();
                } else {
                    write!(
                        stream,
                        "\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .unwrap();
                    stream.write_all(body).unwrap();
                }
                stream.flush().unwrap();
            }
        });
        SmartHttpServer {
            url,
            stop,
            thread: Some(thread),
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct HttpFetchObservation {
        fetched_id: Vec<u8>,
        total_objects: u32,
        received_objects: u32,
        indexed_objects: u32,
        received_bytes_nonzero: bool,
        shallow: bool,
    }

    unsafe fn raw_http_fetch(
        repository: *mut ffi::git_repository,
        url: &core::ffi::CStr,
        depth: Option<i32>,
    ) -> HttpFetchObservation {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create(&mut remote, repository, c"origin".as_ptr(), url.as_ptr())
            },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_fetch_options>() };
        let options = if let Some(depth) = depth {
            assert_eq!(
                unsafe {
                    ffi::git_fetch_options_init(&mut options, ffi::GIT_FETCH_OPTIONS_VERSION)
                },
                0
            );
            options.depth = depth;
            options.download_tags = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL;
            &options
        } else {
            core::ptr::null()
        };
        let status = unsafe {
            ffi::git_remote_fetch(
                remote,
                core::ptr::null(),
                options,
                c"smart HTTP equivalence".as_ptr(),
            )
        };
        if status != 0 {
            let error = unsafe { ffi::git_error_last() };
            let message = if error.is_null() || unsafe { (*error).message }.is_null() {
                "unknown".into()
            } else {
                unsafe { core::ffi::CStr::from_ptr((*error).message) }
                    .to_string_lossy()
                    .into_owned()
            };
            panic!("smart HTTP raw fetch failed ({status}): {message}");
        }
        let stats = unsafe { *ffi::git_remote_stats(remote) };
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut id,
                    repository,
                    c"refs/remotes/origin/master".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        HttpFetchObservation {
            fetched_id: id.id.to_vec(),
            total_objects: stats.total_objects,
            received_objects: stats.received_objects,
            indexed_objects: stats.indexed_objects,
            received_bytes_nonzero: stats.received_bytes > 0,
            shallow: unsafe { ffi::git_repository_is_shallow(repository) } != 0,
        }
    }

    fn safe_http_fetch(
        repository: *mut ffi::git_repository,
        url: &core::ffi::CStr,
        depth: Option<i32>,
    ) -> HttpFetchObservation {
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut remote = git_remote_create(&mut repository_view, c"origin", url).unwrap();
        let mut options = crate::api::remote::GitFetchOptions::new();
        if let Some(depth) = depth {
            options
                .as_mut()
                .set_depth(crate::api::remote::GitFetchDepth::new(depth as _).unwrap());
            options
                .as_mut()
                .set_download_tags(GitRemoteAutotagOption::All);
        }
        git_remote_fetch(
            &mut remote.as_mut(),
            None,
            depth.map(|_| options.as_ref()),
            Some(c"smart HTTP equivalence"),
        )
        .unwrap();
        let stats = git_remote_stats(remote.as_ref());
        let observation = HttpFetchObservation {
            fetched_id: {
                let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
                assert_eq!(
                    unsafe {
                        ffi::git_reference_name_to_id(
                            &mut id,
                            repository,
                            c"refs/remotes/origin/master".as_ptr(),
                        )
                    },
                    0
                );
                id.id.to_vec()
            },
            total_objects: stats.total_objects(),
            received_objects: stats.received_objects(),
            indexed_objects: stats.indexed_objects(),
            received_bytes_nonzero: stats.received_bytes() > 0,
            shallow: unsafe { ffi::git_repository_is_shallow(repository) } != 0,
        };
        observation
    }

    #[test]
    fn io_equiv_smart_http_fetch_downloads_pack_and_updates_refs() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("smart-http-source");
        let server = smart_http_server(&source);
        let raw_directory = TempDir::new("smart-http-raw");
        let safe_directory = TempDir::new("smart-http-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let raw = unsafe { raw_http_fetch(raw_target.as_ptr(), &server.url, None) };
        assert_eq!(
            raw,
            safe_http_fetch(safe_target.as_ptr(), &server.url, None)
        );
        assert!(raw.received_objects >= 10);
        assert!(!raw.shallow);
    }

    #[test]
    fn io_equiv_smart_http_chunked_advertisement_and_pack_responses() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("smart-http-chunked-source");
        let server = smart_http_server_framed(&source, true);
        let raw_directory = TempDir::new("smart-http-chunked-raw");
        let safe_directory = TempDir::new("smart-http-chunked-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let raw = unsafe { raw_http_fetch(raw_target.as_ptr(), &server.url, None) };
        assert_eq!(
            raw,
            safe_http_fetch(safe_target.as_ptr(), &server.url, None)
        );
        assert!(raw.total_objects > 0);
    }

    fn expand_http_history(fixture: &HistoryFixture) {
        for revision in 0..16 {
            std::fs::write(
                fixture.directory.path().join("http-history.txt"),
                format!("smart HTTP revision {revision}\n").repeat(128),
            )
            .unwrap();
            let add = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.directory.path())
                .args(["add", "http-history.txt"])
                .status()
                .unwrap();
            assert!(add.success());
            let commit = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.directory.path())
                .args([
                    "-c",
                    "user.name=Crustify",
                    "-c",
                    "user.email=crustify@example.com",
                    "commit",
                    "-q",
                    "-m",
                    &format!("http revision {revision}"),
                ])
                .env("GIT_AUTHOR_DATE", format!("1700004{revision:03} +0000"))
                .env("GIT_COMMITTER_DATE", format!("1700004{revision:03} +0000"))
                .status()
                .unwrap();
            assert!(commit.success());
        }
    }

    unsafe fn raw_http_unshallow(repository: *mut ffi::git_repository) -> bool {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_lookup(&mut remote, repository, c"origin".as_ptr()) },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_fetch_options>() };
        assert_eq!(
            unsafe { ffi::git_fetch_options_init(&mut options, ffi::GIT_FETCH_OPTIONS_VERSION) },
            0
        );
        options.depth = ffi::git_fetch_depth_t_GIT_FETCH_DEPTH_UNSHALLOW as i32;
        assert_eq!(
            unsafe {
                ffi::git_remote_fetch(
                    remote,
                    core::ptr::null(),
                    &options,
                    c"smart HTTP unshallow".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        unsafe { ffi::git_repository_is_shallow(repository) != 0 }
    }

    fn safe_http_unshallow(repository: *mut ffi::git_repository) -> bool {
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut remote = git_remote_lookup(&mut repository_view, c"origin").unwrap();
        let mut options = crate::api::remote::GitFetchOptions::new();
        options
            .as_mut()
            .set_depth(crate::api::remote::GitFetchDepth::UNSHALLOW);
        git_remote_fetch(
            &mut remote.as_mut(),
            None,
            Some(options.as_ref()),
            Some(c"smart HTTP unshallow"),
        )
        .unwrap();
        unsafe { ffi::git_repository_is_shallow(repository) != 0 }
    }

    #[test]
    fn io_equiv_smart_http_shallow_fetch_limits_history() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("smart-http-shallow-source");
        expand_http_history(&source);
        let server = smart_http_server(&source);
        let raw_directory = TempDir::new("smart-http-shallow-raw");
        let safe_directory = TempDir::new("smart-http-shallow-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let raw = unsafe { raw_http_fetch(raw_target.as_ptr(), &server.url, Some(4)) };
        assert_eq!(
            raw,
            safe_http_fetch(safe_target.as_ptr(), &server.url, Some(4))
        );
        assert!(raw.shallow);
        let raw_shallow = unsafe { raw_http_unshallow(raw_target.as_ptr()) };
        assert_eq!(raw_shallow, safe_http_unshallow(safe_target.as_ptr()));
        assert!(!raw_shallow);
    }

    fn unauthorized_server() -> (std::ffi::CString, std::thread::JoinHandle<Vec<u8>>) {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            use std::io::{Read, Write};

            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            stream
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\nWWW-Authenticate: Basic realm=\"equivalence\"\r\n\r\n",
                )
                .unwrap();
            request
        });
        (
            std::ffi::CString::new(format!("http://{address}/repo.git")).unwrap(),
            handle,
        )
    }

    fn redirect_server(
        destination: &CStr,
    ) -> (std::ffi::CString, std::thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let destination = destination.to_string_lossy().into_owned();
        let handle = std::thread::spawn(move || {
            use std::io::{Read, Write};

            let mut requests = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 1024];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                }
                let line = String::from_utf8_lossy(&request);
                let target = line
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap();
                let suffix = target.strip_prefix("/alias.git").unwrap();
                write!(
                    stream,
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: {destination}{suffix}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                requests.push(request);
            }
            requests
        });
        (
            std::ffi::CString::new(format!("http://{address}/alias.git")).unwrap(),
            handle,
        )
    }

    unsafe fn raw_http_failure(url: &core::ffi::CStr) -> i32 {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_create_detached(&mut remote, url.as_ptr()) },
            0
        );
        let status = unsafe {
            ffi::git_remote_connect(
                remote,
                ffi::git_direction_GIT_DIRECTION_FETCH,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null(),
            )
        };
        unsafe { ffi::git_remote_free(remote) };
        status
    }

    fn safe_http_failure(url: &core::ffi::CStr) -> i32 {
        let mut remote = git_remote_create_detached(url).unwrap();
        git_remote_connect(
            &mut remote.as_mut(),
            crate::util::net::Direction::Fetch,
            None,
            None,
            None,
        )
        .unwrap_err()
    }

    #[test]
    fn io_equiv_http_authentication_failure_and_request() {
        let _libgit2 = Libgit2Init::acquire();
        let (raw_url, raw_server) = unauthorized_server();
        let raw_status = unsafe { raw_http_failure(&raw_url) };
        let raw_request = raw_server.join().unwrap();
        let (safe_url, safe_server) = unauthorized_server();
        let safe_status = safe_http_failure(&safe_url);
        let safe_request = safe_server.join().unwrap();

        assert_eq!(raw_status, safe_status);
        let normalize = |request: &[u8]| {
            let request = String::from_utf8_lossy(request);
            request
                .lines()
                .map(|line| {
                    if line.to_ascii_lowercase().starts_with("host:") {
                        "host: <dynamic>".to_owned()
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(normalize(&raw_request), normalize(&safe_request));
        assert!(raw_request.starts_with(b"GET /repo.git/info/refs?service=git-upload-pack"));
    }

    #[test]
    fn io_equiv_smart_http_follows_absolute_temporary_redirects() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("smart-http-redirect-source");
        let backend = smart_http_server(&source);
        let (redirect, requests) = redirect_server(&backend.url);
        let raw_directory = TempDir::new("smart-http-redirect-raw");
        let safe_directory = TempDir::new("smart-http-redirect-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let raw = unsafe { raw_http_fetch(raw_target.as_ptr(), &redirect, None) };
        assert_eq!(raw, safe_http_fetch(safe_target.as_ptr(), &redirect, None));
        let requests = requests.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| {
            request.starts_with(b"GET /alias.git/info/refs?service=git-upload-pack")
        }));
    }

    fn prepare_http_push_source(fixture: &HistoryFixture) {
        std::fs::write(
            fixture.directory.path().join("http-push.txt"),
            b"smart HTTP push payload\n",
        )
        .unwrap();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["add", "http-push.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args([
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "smart HTTP push commit",
            ])
            .env("GIT_AUTHOR_DATE", "1700006000 +0000")
            .env("GIT_COMMITTER_DATE", "1700006000 +0000")
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn io_equiv_smart_http_push_create_delete_and_upload() {
        let _libgit2 = Libgit2Init::acquire();
        let destination = HistoryFixture::new("smart-http-push-destination");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(destination.directory.path())
            .args(["config", "http.receivepack", "true"])
            .status()
            .unwrap();
        assert!(status.success());
        let server = smart_http_server(&destination);
        let raw = HistoryFixture::new("smart-http-push-raw");
        let safe = HistoryFixture::new("smart-http-push-safe");
        prepare_http_push_source(&raw);
        prepare_http_push_source(&safe);
        unsafe { raw_push(raw.repository.as_ptr(), &server.url) };
        let mut safe_repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(safe.repository.as_ptr()) }
                .unwrap();
        safe_push(&mut safe_repository, &server.url);
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(destination.directory.path())
            .args(["show", "refs/heads/copied:http-push.txt"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"smart HTTP push payload\n");
    }

    unsafe fn raw_http_push_once(repository: *mut ffi::git_repository, url: &CStr) -> i32 {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create(&mut remote, repository, c"reject".as_ptr(), url.as_ptr())
            },
            0
        );
        let mut spec = c"refs/heads/master:refs/heads/master".as_ptr().cast_mut();
        let refspecs = ffi::git_strarray {
            strings: &mut spec,
            count: 1,
        };
        let status = unsafe { ffi::git_remote_push(remote, &refspecs, core::ptr::null()) };
        unsafe { ffi::git_remote_free(remote) };
        status
    }

    fn safe_http_push_once(repository: *mut ffi::git_repository, url: &CStr) -> i32 {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut remote = git_remote_create(&mut repository, c"reject", url).unwrap();
        let mut spec = c"refs/heads/master:refs/heads/master".as_ptr().cast_mut();
        let mut raw_refspecs = ffi::git_strarray {
            strings: &mut spec,
            count: 1,
        };
        let refspecs = unsafe { GitStrArrayRef::from_ptr(&mut raw_refspecs) }.unwrap();
        match git_remote_push(&mut remote.as_mut(), Some(refspecs), None) {
            Ok(()) => 0,
            Err(status) => status,
        }
    }

    fn rewind_source(fixture: &HistoryFixture) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.directory.path())
            .args(["reset", "--hard", "-q", "HEAD~1"])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn io_equiv_smart_http_rejects_non_fast_forward_push() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_destination = HistoryFixture::new("http-push-reject-destination-raw");
        let safe_destination = HistoryFixture::new("http-push-reject-destination-safe");
        for destination in [&raw_destination, &safe_destination] {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(destination.directory.path())
                .args(["config", "http.receivepack", "true"])
                .status()
                .unwrap();
            assert!(status.success());
        }
        let raw_server = smart_http_server(&raw_destination);
        let safe_server = smart_http_server(&safe_destination);
        let raw_source = HistoryFixture::new("http-push-reject-source-raw");
        let safe_source = HistoryFixture::new("http-push-reject-source-safe");
        rewind_source(&raw_source);
        rewind_source(&safe_source);
        let raw = unsafe { raw_http_push_once(raw_source.repository.as_ptr(), &raw_server.url) };
        let safe = safe_http_push_once(safe_source.repository.as_ptr(), &safe_server.url);
        assert_eq!(raw, safe);
        assert_eq!(raw, ffi::git_error_code_GIT_ENONFASTFORWARD);
        let raw_head = std::process::Command::new("git")
            .arg("-C")
            .arg(raw_destination.directory.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let safe_head = std::process::Command::new("git")
            .arg("-C")
            .arg(safe_destination.directory.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert_eq!(raw_head.stdout, safe_head.stdout);
    }

    fn pkt_line(payload: &[u8]) -> Vec<u8> {
        let mut packet = format!("{:04x}", payload.len() + 4).into_bytes();
        packet.extend_from_slice(payload);
        packet
    }

    fn advertisement_server() -> (std::ffi::CString, std::thread::JoinHandle<Vec<u8>>) {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            use std::io::{Read, Write};

            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let oid = b"0123456789012345678901234567890123456789";
            let mut body = pkt_line(b"# service=git-upload-pack\n");
            body.extend_from_slice(b"0000");
            let mut head = oid.to_vec();
            head.extend_from_slice(b" HEAD\0symref=HEAD:refs/heads/main agent=crustify-equiv/1\n");
            body.extend_from_slice(&pkt_line(&head));
            let mut main = oid.to_vec();
            main.extend_from_slice(b" refs/heads/main\n");
            body.extend_from_slice(&pkt_line(&main));
            body.extend_from_slice(b"0000");
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-git-upload-pack-advertisement\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
            request
        });
        (
            std::ffi::CString::new(format!("http://{address}/repo.git")).unwrap(),
            handle,
        )
    }

    unsafe fn raw_http_advertisement(url: &core::ffi::CStr) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_create_detached(&mut remote, url.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_connect(
                    remote,
                    ffi::git_direction_GIT_DIRECTION_FETCH,
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut heads = core::ptr::null_mut();
        let mut count = 0;
        assert_eq!(
            unsafe { ffi::git_remote_ls(&mut heads, &mut count, remote) },
            0
        );
        let observed = (0..count)
            .map(|index| {
                let head = unsafe { *heads.add(index) };
                (
                    unsafe { core::ffi::CStr::from_ptr((*head).name) }
                        .to_bytes()
                        .to_vec(),
                    unsafe { (*head).oid.id.to_vec() },
                )
            })
            .collect();
        assert_eq!(unsafe { ffi::git_remote_disconnect(remote) }, 0);
        unsafe { ffi::git_remote_free(remote) };
        observed
    }

    fn safe_http_advertisement(url: &core::ffi::CStr) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut remote = git_remote_create_detached(url).unwrap();
        let observed = {
            let mut remote_view = remote.as_mut();
            git_remote_connect(
                &mut remote_view,
                crate::util::net::Direction::Fetch,
                None,
                None,
                None,
            )
            .unwrap();
            let heads = git_remote_ls(&mut remote_view).unwrap();
            (0..heads.len())
                .map(|index| {
                    let head = heads.get(index).unwrap();
                    (
                        head.name().unwrap().to_bytes().to_vec(),
                        head.oid().raw_bytes().elems().collect(),
                    )
                })
                .collect()
        };
        git_remote_disconnect(&mut remote.as_mut()).unwrap();
        observed
    }

    #[test]
    fn io_equiv_smart_http_advertisement_parsing() {
        let _libgit2 = Libgit2Init::acquire();
        let (raw_url, raw_server) = advertisement_server();
        let raw = unsafe { raw_http_advertisement(&raw_url) };
        let raw_request = raw_server.join().unwrap();
        let (safe_url, safe_server) = advertisement_server();
        let safe = safe_http_advertisement(&safe_url);
        let safe_request = safe_server.join().unwrap();
        assert_eq!(raw, safe);
        assert_eq!(
            raw.iter().map(|head| head.0.as_slice()).collect::<Vec<_>>(),
            [b"HEAD".as_slice(), b"refs/heads/main".as_slice()]
        );
        assert_eq!(
            raw_request.split(|byte| *byte == b'\n').next(),
            safe_request.split(|byte| *byte == b'\n').next()
        );
    }

    unsafe fn prepare_prunable_remote(
        target: *mut ffi::git_repository,
        source: *mut ffi::git_repository,
        url: &core::ffi::CStr,
    ) {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create_with_fetchspec(
                    &mut remote,
                    target,
                    c"origin".as_ptr(),
                    url.as_ptr(),
                    c"+refs/heads/*:refs/remotes/origin/*".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_fetch(
                    remote,
                    core::ptr::null(),
                    core::ptr::null(),
                    c"initial fetch".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        assert_eq!(
            unsafe { ffi::git_reference_remove(source, c"refs/heads/topic".as_ptr()) },
            0
        );
    }

    #[derive(Debug, Eq, PartialEq)]
    struct PruneObservation {
        topic_status: i32,
        master: Vec<u8>,
        tag: Vec<u8>,
        prune_refs: bool,
        autotag: ffi::git_remote_autotag_option_t,
        remotes_after_delete: usize,
    }

    unsafe fn raw_prune(repository: *mut ffi::git_repository) -> PruneObservation {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_lookup(&mut remote, repository, c"origin".as_ptr()) },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_fetch_options>() };
        assert_eq!(
            unsafe { ffi::git_fetch_options_init(&mut options, ffi::GIT_FETCH_OPTIONS_VERSION) },
            0
        );
        options.prune = ffi::git_fetch_prune_t_GIT_FETCH_PRUNE;
        options.download_tags = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL;
        assert_eq!(
            unsafe {
                ffi::git_remote_fetch(
                    remote,
                    core::ptr::null(),
                    &options,
                    c"pruning fetch".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_connect(
                    remote,
                    ffi::git_direction_GIT_DIRECTION_FETCH,
                    core::ptr::null(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_remote_prune(remote, core::ptr::null()) },
            0
        );
        assert_eq!(unsafe { ffi::git_remote_disconnect(remote) }, 0);
        let prune_refs = unsafe { ffi::git_remote_prune_refs(remote) != 0 };
        let autotag = unsafe { ffi::git_remote_autotag(remote) };
        unsafe { ffi::git_remote_free(remote) };
        let mut topic = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let topic_status = unsafe {
            ffi::git_reference_name_to_id(
                &mut topic,
                repository,
                c"refs/remotes/origin/topic".as_ptr(),
            )
        };
        let mut master = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let mut tag = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut master,
                    repository,
                    c"refs/remotes/origin/master".as_ptr(),
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(&mut tag, repository, c"refs/tags/v1.0".as_ptr())
            },
            0
        );
        assert_eq!(
            unsafe { ffi::git_remote_delete(repository, c"origin".as_ptr()) },
            0
        );
        let mut remotes = unsafe { core::mem::zeroed::<ffi::git_strarray>() };
        assert_eq!(unsafe { ffi::git_remote_list(&mut remotes, repository) }, 0);
        let remotes_after_delete = remotes.count;
        unsafe { ffi::git_strarray_dispose(&mut remotes) };
        PruneObservation {
            topic_status,
            master: master.id.to_vec(),
            tag: tag.id.to_vec(),
            prune_refs,
            autotag,
            remotes_after_delete,
        }
    }

    fn safe_prune(repository: *mut ffi::git_repository) -> PruneObservation {
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut remote = git_remote_lookup(&mut repository, c"origin").unwrap();
        let mut options = crate::api::remote::GitFetchOptions::new();
        options.as_mut().set_prune(GitFetchPrune::Prune);
        options
            .as_mut()
            .set_download_tags(GitRemoteAutotagOption::All);
        git_remote_fetch(
            &mut remote.as_mut(),
            None,
            Some(options.as_ref()),
            Some(c"pruning fetch"),
        )
        .unwrap();
        git_remote_connect(
            &mut remote.as_mut(),
            crate::util::net::Direction::Fetch,
            None,
            None,
            None,
        )
        .unwrap();
        git_remote_prune(&mut remote.as_mut(), None).unwrap();
        git_remote_disconnect(&mut remote.as_mut()).unwrap();
        let prune_refs = git_remote_prune_refs(remote.as_ref());
        let autotag = git_remote_autotag(remote.as_ref()).unwrap().into();
        drop(remote);
        let mut topic = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let topic_status = unsafe {
            ffi::git_reference_name_to_id(
                &mut topic,
                repository.as_mut_ptr(),
                c"refs/remotes/origin/topic".as_ptr(),
            )
        };
        let mut master =
            crate::refs::git_reference_name_to_id(&mut repository, c"refs/remotes/origin/master")
                .unwrap();
        let mut tag =
            crate::refs::git_reference_name_to_id(&mut repository, c"refs/tags/v1.0").unwrap();
        let bytes = |value: &mut crate::oid::Oid| {
            unsafe { crate::oid::OidRef::from_ptr(core::ptr::from_mut(value).cast()) }
                .unwrap()
                .raw_bytes()
                .elems()
                .collect()
        };
        let master = bytes(&mut master);
        let tag = bytes(&mut tag);
        git_remote_delete(&mut repository, c"origin").unwrap();
        let remotes_after_delete = git_remote_list(&mut repository).unwrap().as_ref().count();
        PruneObservation {
            topic_status,
            master,
            tag,
            prune_refs,
            autotag,
            remotes_after_delete,
        }
    }

    #[test]
    fn io_equiv_remote_fetch_prune_tags_explicit_prune_and_delete() {
        let _libgit2 = Libgit2Init::acquire();
        let raw_source = HistoryFixture::new("prune-source-raw");
        let safe_source = HistoryFixture::new("prune-source-safe");
        let raw_directory = TempDir::new("prune-target-raw");
        let safe_directory = TempDir::new("prune-target-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let raw_url = file_url(raw_source.directory.path());
        let safe_url = file_url(safe_source.directory.path());
        unsafe {
            prepare_prunable_remote(
                raw_target.as_ptr(),
                raw_source.repository.as_ptr(),
                &raw_url,
            );
            prepare_prunable_remote(
                safe_target.as_ptr(),
                safe_source.repository.as_ptr(),
                &safe_url,
            );
        }
        let raw = unsafe { raw_prune(raw_target.as_ptr()) };
        assert_eq!(raw, safe_prune(safe_target.as_ptr()));
        assert_eq!(raw.topic_status, ffi::git_error_code_GIT_ENOTFOUND);
        assert_eq!(raw.remotes_after_delete, 0);
    }

    unsafe fn raw_remote_construction(
        repository: *mut ffi::git_repository,
        url: &CStr,
    ) -> Vec<(Option<Vec<u8>>, Vec<u8>, Option<Vec<u8>>, bool, usize)> {
        let mut observations = Vec::new();
        let mut anonymous = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_create_anonymous(&mut anonymous, repository, url.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_set_instance_url(anonymous, c"file:///instance-fetch".as_ptr())
            },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_remote_set_instance_pushurl(anonymous, c"file:///instance-push".as_ptr())
            },
            0
        );
        observations.push(unsafe { raw_remote_shape(anonymous) });
        unsafe { ffi::git_remote_free(anonymous) };

        let mut detached = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_create_detached(&mut detached, url.as_ptr()) },
            0
        );
        observations.push(unsafe { raw_remote_shape(detached) });
        unsafe { ffi::git_remote_free(detached) };

        let mut options = unsafe { core::mem::zeroed::<ffi::git_remote_create_options>() };
        assert_eq!(
            unsafe {
                ffi::git_remote_create_options_init(
                    &mut options,
                    ffi::GIT_REMOTE_CREATE_OPTIONS_VERSION,
                )
            },
            0
        );
        options.repository = repository;
        options.name = c"constructed".as_ptr();
        options.fetchspec = c"+refs/heads/*:refs/remotes/constructed/*".as_ptr();
        let mut configured = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_remote_create_with_opts(&mut configured, url.as_ptr(), &options) },
            0
        );
        observations.push(unsafe { raw_remote_shape(configured) });
        unsafe { ffi::git_remote_free(configured) };
        observations
    }

    unsafe fn raw_remote_shape(
        remote: *mut ffi::git_remote,
    ) -> (Option<Vec<u8>>, Vec<u8>, Option<Vec<u8>>, bool, usize) {
        let optional = |value: *const core::ffi::c_char| {
            (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_bytes().to_vec())
        };
        (
            optional(unsafe { ffi::git_remote_name(remote) }),
            optional(unsafe { ffi::git_remote_url(remote) }).unwrap(),
            optional(unsafe { ffi::git_remote_pushurl(remote) }),
            !unsafe { ffi::git_remote_owner(remote) }.is_null(),
            unsafe { ffi::git_remote_refspec_count(remote) },
        )
    }

    fn safe_remote_shape(
        remote: GitRemoteRef<'_>,
    ) -> (Option<Vec<u8>>, Vec<u8>, Option<Vec<u8>>, bool, usize) {
        (
            git_remote_name(remote).map(|value| value.to_bytes().to_vec()),
            git_remote_url(remote).unwrap().to_bytes().to_vec(),
            git_remote_pushurl(remote).map(|value| value.to_bytes().to_vec()),
            git_remote_owner(remote).is_some(),
            git_remote_refspec_count(remote),
        )
    }

    fn safe_remote_construction(
        repository: *mut ffi::git_repository,
        url: &CStr,
    ) -> Vec<(Option<Vec<u8>>, Vec<u8>, Option<Vec<u8>>, bool, usize)> {
        let mut observations = Vec::new();
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut anonymous = git_remote_create_anonymous(&mut repository_view, url).unwrap();
        git_remote_set_instance_url(&mut anonymous.as_mut(), c"file:///instance-fetch").unwrap();
        git_remote_set_instance_pushurl(&mut anonymous.as_mut(), c"file:///instance-push").unwrap();
        observations.push(safe_remote_shape(anonymous.as_ref()));
        drop(anonymous);

        let detached = git_remote_create_detached(url).unwrap();
        observations.push(safe_remote_shape(detached.as_ref()));
        drop(detached);

        let mut options =
            git_remote_create_options_init(ffi::GIT_REMOTE_CREATE_OPTIONS_VERSION).unwrap();
        {
            let mut view = options.as_mut();
            unsafe {
                view.set_borrowed_repository(Some(
                    crate::repository::GitRepositoryMut::from_ptr(repository).unwrap(),
                ));
                view.set_borrowed_name(Some(c"constructed"));
                view.set_borrowed_fetchspec(Some(c"+refs/heads/*:refs/remotes/constructed/*"));
            }
        }
        let configured = git_remote_create_with_opts(url, options.as_ref()).unwrap();
        observations.push(safe_remote_shape(configured.as_ref()));
        observations
    }

    #[test]
    fn io_equiv_remote_anonymous_detached_options_and_instance_urls() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("remote-construction-source");
        let raw = HistoryFixture::new("remote-construction-raw");
        let safe = HistoryFixture::new("remote-construction-safe");
        let url = file_url(source.directory.path());
        let raw_observation = unsafe { raw_remote_construction(raw.repository.as_ptr(), &url) };
        let safe_observation = safe_remote_construction(safe.repository.as_ptr(), &url);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation[0].0, None);
        assert_eq!(
            raw_observation[2].0.as_deref(),
            Some(b"constructed".as_slice())
        );
        assert!(raw_observation[2].3);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct DownloadObservation {
        received: u32,
        indexed: u32,
        connected_during_download: bool,
        tracking_id: Vec<u8>,
        fetch_head_exists: bool,
    }

    unsafe fn raw_download_then_update(
        repository: *mut ffi::git_repository,
        url: &CStr,
    ) -> DownloadObservation {
        let mut remote = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_remote_create_with_fetchspec(
                    &mut remote,
                    repository,
                    c"origin".as_ptr(),
                    url.as_ptr(),
                    c"+refs/heads/*:refs/remotes/origin/*".as_ptr(),
                )
            },
            0
        );
        unsafe { ffi::git_remote_free(remote) };
        assert_eq!(
            unsafe { ffi::git_remote_set_url(repository, c"origin".as_ptr(), url.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_remote_lookup(&mut remote, repository, c"origin".as_ptr()) },
            0
        );
        let mut options = unsafe { core::mem::zeroed::<ffi::git_fetch_options>() };
        assert_eq!(
            unsafe { ffi::git_fetch_options_init(&mut options, ffi::GIT_FETCH_OPTIONS_VERSION) },
            0
        );
        options.download_tags = ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL;
        assert_eq!(
            unsafe { ffi::git_remote_download(remote, core::ptr::null(), &options) },
            0
        );
        let connected_during_download = unsafe { ffi::git_remote_connected(remote) != 0 };
        let stats = unsafe { ffi::git_remote_stats(remote) };
        let received = unsafe { (*stats).received_objects };
        let indexed = unsafe { (*stats).indexed_objects };
        assert_eq!(
            unsafe {
                ffi::git_remote_update_tips(
                    remote,
                    core::ptr::null(),
                    ffi::git_remote_update_flags_GIT_REMOTE_UPDATE_FETCHHEAD
                        | ffi::git_remote_update_flags_GIT_REMOTE_UPDATE_REPORT_UNCHANGED,
                    ffi::git_remote_autotag_option_t_GIT_REMOTE_DOWNLOAD_TAGS_ALL,
                    c"separate download and update".as_ptr(),
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_remote_disconnect(remote) }, 0);
        assert_eq!(unsafe { ffi::git_remote_stop(remote) }, 0);
        unsafe { ffi::git_remote_free(remote) };
        let mut tracking = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut tracking,
                    repository,
                    c"refs/remotes/origin/master".as_ptr(),
                )
            },
            0
        );
        let repo_path = unsafe { CStr::from_ptr(ffi::git_repository_path(repository)) };
        let fetch_head_exists = std::path::Path::new(repo_path.to_str().unwrap())
            .join("FETCH_HEAD")
            .exists();
        DownloadObservation {
            received,
            indexed,
            connected_during_download,
            tracking_id: tracking.id.to_vec(),
            fetch_head_exists,
        }
    }

    fn safe_download_then_update(
        repository: *mut ffi::git_repository,
        url: &CStr,
    ) -> DownloadObservation {
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let remote = git_remote_create_with_fetchspec(
            &mut repository_view,
            c"origin",
            url,
            Some(c"+refs/heads/*:refs/remotes/origin/*"),
        )
        .unwrap();
        drop(remote);
        git_remote_set_url(&mut repository_view, c"origin", Some(url)).unwrap();
        let mut remote = git_remote_lookup(&mut repository_view, c"origin").unwrap();
        let mut options = crate::api::remote::GitFetchOptions::new();
        options
            .as_mut()
            .set_download_tags(GitRemoteAutotagOption::All);
        git_remote_download(&mut remote.as_mut(), None, Some(options.as_ref())).unwrap();
        let connected_during_download = git_remote_connected(remote.as_ref());
        let stats = git_remote_stats(remote.as_ref());
        let received = stats.received_objects();
        let indexed = stats.indexed_objects();
        git_remote_update_tips(
            &mut remote.as_mut(),
            None,
            GitRemoteUpdateFlags::ALL,
            GitRemoteAutotagOption::All,
            Some(c"separate download and update"),
        )
        .unwrap();
        git_remote_disconnect(&mut remote.as_mut()).unwrap();
        git_remote_stop(&mut remote.as_mut()).unwrap();
        drop(remote);
        let mut tracking = crate::refs::git_reference_name_to_id(
            &mut repository_view,
            c"refs/remotes/origin/master",
        )
        .unwrap();
        let tracking = unsafe { crate::oid::OidRef::from_ptr((&raw mut tracking).cast()) }
            .unwrap()
            .raw_bytes()
            .elems()
            .collect();
        let repo_path = crate::repository::git_repository_path(repository_view.as_ref()).unwrap();
        let fetch_head_exists = std::path::Path::new(repo_path.to_str().unwrap())
            .join("FETCH_HEAD")
            .exists();
        DownloadObservation {
            received,
            indexed,
            connected_during_download,
            tracking_id: tracking,
            fetch_head_exists,
        }
    }

    #[test]
    fn io_equiv_remote_separate_download_update_tips_and_stop() {
        let _libgit2 = Libgit2Init::acquire();
        let source = HistoryFixture::new("remote-download-source");
        let raw_directory = TempDir::new("remote-download-raw");
        let safe_directory = TempDir::new("remote-download-safe");
        let raw_target = RawRepository::init(&raw_directory.c_path(), true).unwrap();
        let safe_target = RawRepository::init(&safe_directory.c_path(), true).unwrap();
        let url = file_url(source.directory.path());
        let raw = unsafe { raw_download_then_update(raw_target.as_ptr(), &url) };
        assert_eq!(raw, safe_download_then_update(safe_target.as_ptr(), &url));
        assert!(raw.connected_during_download);
        assert!(raw.fetch_head_exists);
        assert!(raw.received > 0);
    }
}
