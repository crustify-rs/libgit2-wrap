//! Safe wrappers for libgit2 object APIs.

use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::types::GitObjectType;
use crate::ffi;
use crate::filter::{GitFilterListMut, GitFilterListRef};
use crate::oid::{InvalidOidType, OidType};

/// Wraps: git_object_id_options
/// Layout-compatible object-ID options that exclusively borrow their optional
/// filter list for `'data`.
///
/// The borrow is exclusive rather than shared: computing an object ID applies
/// the installed list, and applying a filter writes back into the list's own
/// entries -- `crlf_apply` fills a missing entry payload through the
/// `void **payload` slot `setup_stream` hands it. The list is therefore
/// installed by moving in its [`GitFilterListMut`], which parks that exclusive
/// borrow in the options for `'data`.
///
/// The data lifetime is invariant: a mutable handle can install a filter list
/// and a later shared handle can borrow it back. Invariance prevents safe code
/// from shrinking `'data` while setting the field and then observing a stale
/// pointer through a longer-lived handle.
#[repr(transparent)]
pub struct GitObjectIdOptions<'data> {
    inner: CType<ffi::git_object_id_options>,
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitObjectIdOptions`].
#[repr(transparent)]
pub struct GitObjectIdOptionsRef<'object, 'data>(CPtr<'object, GitObjectIdOptions<'data>>);

impl Clone for GitObjectIdOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitObjectIdOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitObjectIdOptions`].
#[repr(transparent)]
pub struct GitObjectIdOptionsMut<'object, 'data>(GitObjectIdOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitObjectIdOptions<'data> {
    type C = ffi::git_object_id_options;
    type Ref<'object>
        = GitObjectIdOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitObjectIdOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitObjectIdOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitObjectIdOptionsMut(GitObjectIdOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this options header only borrows its filter list and owns no
// resource, so disposing inline storage requires no action.
unsafe impl CValued for GitObjectIdOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitObjectIdOptions<'data> {
    /// Constructs options equivalent to `GIT_OBJECT_ID_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every bindgen field admits the all-zero bit pattern. The
        // required ABI version is installed before the value is returned.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        options
            .as_mut()
            .set_version(ffi::GIT_OBJECT_ID_OPTIONS_VERSION);
        options
    }
}

impl<'object, 'data> GitObjectIdOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options live for `'object`. A non-null
    /// filter list must remain live and shared-only for `'data`, which must
    /// outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_object_id_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitObjectIdOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_object_id_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_object_id_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: raw-place projection copies the initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_object_id_options.oid_type
    /// Returns the selected object-ID format, or `None` for libgit2's default.
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            OidType::try_from(raw).map(Some)
        }
    }

    /// Field: git_object_id_options.filters
    /// Borrows the installed filter list for read-only inspection.
    ///
    /// The options hold the list's exclusive borrow, so this shared reborrow
    /// only supports queries such as `git_filter_list_contains`; applying the
    /// list needs [`GitObjectIdOptionsMut::filters_mut`].
    #[must_use]
    pub fn filters(&self) -> Option<GitFilterListRef<'object>> {
        // SAFETY: raw-place projection copies the initialized pointer field.
        let filters = unsafe { addr_of!((*self.as_ptr()).filters).read() };
        // SAFETY: the wrapper contract keeps a non-null filter list live for
        // at least this options borrow, and the exclusive borrow parked in the
        // options makes this shared reborrow the only reachable handle.
        unsafe { GitFilterListRef::from_ptr(filters) }
    }

    /// Field: git_object_id_options.object_type
    /// Returns the configured object kind, or `None` for the default blob.
    pub fn object_type(&self) -> Result<Option<GitObjectType>, ffi::git_object_t> {
        // SAFETY: raw-place projection copies the initialized scalar field.
        let raw = unsafe { addr_of!((*self.as_ptr()).object_type).read() };
        if raw == 0 {
            Ok(None)
        } else {
            GitObjectType::from_raw(raw).map(Some).ok_or(raw)
        }
    }
}

impl<'object, 'data> GitObjectIdOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path to the
    /// options value may be used for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_object_id_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitObjectIdOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_object_id_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitObjectIdOptionsRef<'_, 'data> {
        GitObjectIdOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects an object-ID format, or the libgit2 default with `None`.
    pub fn set_oid_type(&mut self, oid_type: Option<OidType>) {
        let raw = oid_type.map_or(0, Into::into);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(raw) }
    }

    /// Installs an optional exclusively borrowed filter list.
    ///
    /// Moving the exclusive handle in parks the list's `'data` borrow here:
    /// libgit2 mutates the list while applying it, so no other handle to it
    /// may exist while these options can still be used.
    ///
    /// A second handle to an installed list is therefore rejected:
    ///
    /// ```compile_fail
    /// use libgit2::api::object::GitObjectIdOptions;
    /// use libgit2::filter::RepositoryFilterList;
    ///
    /// fn install(list: &mut RepositoryFilterList<'_>) {
    ///     let mut options = GitObjectIdOptions::new();
    ///     options.as_mut().set_filters(Some(list.as_mut()));
    ///     let _second = list.as_mut();
    ///     let _still_installed = options.as_ref();
    /// }
    /// ```
    pub fn set_filters(&mut self, filters: Option<GitFilterListMut<'data>>) {
        let filters = filters.map_or(core::ptr::null_mut(), |mut filters| filters.as_mut_ptr());
        // SAFETY: this exclusive handle permits the pointer write, and the
        // invariant wrapper lifetime keeps a non-null list alive and reachable
        // only through these options.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).filters).write(filters) }
    }

    /// Reborrows the installed filter list exclusively.
    #[must_use]
    pub fn filters_mut(&mut self) -> Option<GitFilterListMut<'_>> {
        // SAFETY: this exclusive handle permits the raw-place pointer read.
        let filters = unsafe { addr_of!((*self.as_mut_ptr()).filters).read() };
        // SAFETY: a non-null list was installed by moving in its exclusive
        // handle, so reborrowing it through this exclusive options handle
        // produces the only live path to the list.
        unsafe { GitFilterListMut::from_ptr(filters) }
    }

    /// Selects an object kind, or the default blob with `None`.
    pub fn set_object_type(&mut self, object_type: Option<GitObjectType>) {
        let raw = object_type.map_or(0, Into::into);
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).object_type).write(raw) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitObjectIdOptions<'static>>(),
            size_of::<ffi::git_object_id_options>()
        );
        assert_eq!(
            align_of::<GitObjectIdOptions<'static>>(),
            align_of::<ffi::git_object_id_options>()
        );
        assert_eq!(
            size_of::<GitObjectIdOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_object_id_options>()
        );

        let mut options = GitObjectIdOptions::new();
        {
            let mut view = options.as_mut();
            assert_eq!(view.as_ref().version(), ffi::GIT_OBJECT_ID_OPTIONS_VERSION);
            assert_eq!(view.as_ref().oid_type(), Ok(None));
            assert_eq!(view.as_ref().object_type(), Ok(None));
            assert!(view.as_ref().filters().is_none());

            view.set_oid_type(Some(OidType::Sha256));
            view.set_object_type(Some(GitObjectType::BLOB));
            view.set_filters(None);

            assert_eq!(view.as_ref().oid_type(), Ok(Some(OidType::Sha256)));
            assert_eq!(view.as_ref().object_type(), Ok(Some(GitObjectType::BLOB)));
            assert!(view.as_ref().filters().is_none());
        }
    }
}

#[cfg(test)]
mod filtered_object_id_tests {
    use super::*;

    use crate::api::filter::{GitFilterFlags, GitFilterMode};
    use crate::filter::git_filter_list_load;
    use crate::object::git_object_id_from_buffer;
    use crate::oid::{Oid, OidRef, git_oid_equal};
    use crate::repository::git_repository_open;

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
            // SAFETY: balances this guard's initialization; the repository and
            // filter list opened under it are released first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built repository whose working directory carries the single
    /// `.gitattributes` entry that installs the CRLF filter for one path.
    struct WorkdirRepo(std::path::PathBuf);

    impl WorkdirRepo {
        fn create() -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-objid-filters-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            let git = path.join(".git");
            std::fs::create_dir_all(git.join("objects")).expect("a loose-object directory");
            std::fs::create_dir_all(git.join("refs/heads")).expect("a refs directory");
            std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                git.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
            )
            .expect("a config file");
            std::fs::write(path.join(".gitattributes"), b"report.txt text eol=lf\n")
                .expect("an attributes file");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for WorkdirRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn oid_ref(oid: &mut Oid) -> OidRef<'_> {
        // SAFETY: `oid` is live initialized layout-compatible storage, and the
        // exclusive borrow keeps it still for the returned handle.
        unsafe { OidRef::from_ptr(core::ptr::from_mut(oid).cast::<ffi::git_oid>()) }
            .expect("a borrowed local is non-null")
    }

    #[test]
    fn an_installed_filter_list_is_applied_while_hashing() {
        let _init = Libgit2Init::acquire();
        let directory = WorkdirRepo::create();
        let repository =
            git_repository_open(&directory.c_path()).expect("the hand-built directory opens");

        let mut filters = git_filter_list_load(
            repository.as_ref(),
            None,
            c"report.txt",
            GitFilterMode::ToObjectDatabase,
            GitFilterFlags::DEFAULT,
        )
        .expect("loading the working-directory filters")
        .expect("the `text` attribute installs the CRLF filter");

        let mut options = GitObjectIdOptions::new();
        // Installing moves in the list's exclusive handle, because the C apply
        // path writes back into the list's own entries.
        options.as_mut().set_filters(Some(filters.as_mut()));
        assert!(options.as_ref().filters().is_some());
        assert!(options.as_mut().filters_mut().is_some());

        let mut filtered =
            git_object_id_from_buffer(b"first\r\nsecond\r\n", Some(options.as_ref()))
                .expect("hashing through the installed filter list");
        let mut normalized = git_object_id_from_buffer(b"first\nsecond\n", None)
            .expect("hashing the normalized content");
        let mut unfiltered = git_object_id_from_buffer(b"first\r\nsecond\r\n", None)
            .expect("hashing the raw content");

        assert!(git_oid_equal(
            oid_ref(&mut filtered),
            oid_ref(&mut normalized)
        ));
        assert!(!git_oid_equal(
            oid_ref(&mut filtered),
            oid_ref(&mut unfiltered)
        ));

        // A second hash reuses the entry payload the first apply installed in
        // the list, so the parked exclusive borrow has to survive the call.
        let mut again = git_object_id_from_buffer(b"first\r\nsecond\r\n", Some(options.as_ref()))
            .expect("hashing through the same list a second time");
        assert!(git_oid_equal(oid_ref(&mut filtered), oid_ref(&mut again)));
    }
}
