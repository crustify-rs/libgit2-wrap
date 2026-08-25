//! Safe wrappers for libgit2 commit APIs.

use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CSlice, CVal, CValued};

/// Borrowed view over the parent-pointer array passed to commit callbacks.
#[derive(Clone, Copy)]
pub struct GitCommitParents<'a> {
    ptr: NonNull<*const crate::ffi::git_commit>,
    len: usize,
    _borrow: PhantomData<crate::commit::GitCommitRef<'a>>,
}

impl<'a> GitCommitParents<'a> {
    /// Constructs the transient view used by a callback trampoline.
    ///
    /// # Safety
    ///
    /// `ptr` must address `len` readable pointers to live commits for `'a`,
    /// and every element must be non-null.
    pub(crate) unsafe fn from_raw(
        ptr: *const *const crate::ffi::git_commit,
        len: usize,
    ) -> Option<Self> {
        let ptr = if len == 0 && ptr.is_null() {
            NonNull::dangling()
        } else {
            NonNull::new(ptr.cast_mut())?
        };
        Some(Self {
            ptr,
            len,
            _borrow: PhantomData,
        })
    }

    /// Returns the number of parent commits.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether there are no parents.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one parent by position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<crate::commit::GitCommitRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: construction guarantees a readable `len`-element pointer
        // array, and the bounds check selects one initialized element.
        let parent = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: construction requires each element to name a live commit for
        // `'a`; a null element is defensively rejected.
        unsafe { crate::commit::GitCommitRef::from_ptr(parent.cast_mut()) }
    }
}

/// Wraps: git_commit_create_cb
/// Safe callable surface for custom commit creation during a rebase.
pub trait GitCommitCreateCallback {
    /// Creates a commit ID or returns `GIT_PASSTHROUGH`/another error code.
    #[allow(clippy::too_many_arguments)]
    fn create(
        &mut self,
        out: &mut crate::oid::OidMut<'_>,
        author: crate::api::types::GitSignatureRef<'_>,
        committer: crate::api::types::GitSignatureRef<'_>,
        message_encoding: Option<&core::ffi::CStr>,
        message: &core::ffi::CStr,
        tree: crate::tree::GitTreeRef<'_>,
        parents: GitCommitParents<'_>,
    ) -> i32;
}

impl<F> GitCommitCreateCallback for F
where
    F: FnMut(
        &mut crate::oid::OidMut<'_>,
        crate::api::types::GitSignatureRef<'_>,
        crate::api::types::GitSignatureRef<'_>,
        Option<&core::ffi::CStr>,
        &core::ffi::CStr,
        crate::tree::GitTreeRef<'_>,
        GitCommitParents<'_>,
    ) -> i32,
{
    fn create(
        &mut self,
        out: &mut crate::oid::OidMut<'_>,
        author: crate::api::types::GitSignatureRef<'_>,
        committer: crate::api::types::GitSignatureRef<'_>,
        message_encoding: Option<&core::ffi::CStr>,
        message: &core::ffi::CStr,
        tree: crate::tree::GitTreeRef<'_>,
        parents: GitCommitParents<'_>,
    ) -> i32 {
        self(
            out,
            author,
            committer,
            message_encoding,
            message,
            tree,
            parents,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_commit_creation() {
        fn accepts<C: GitCommitCreateCallback>(_callback: C) {}
        accepts(
            |_: &mut crate::oid::OidMut<'_>,
             _: crate::api::types::GitSignatureRef<'_>,
             _: crate::api::types::GitSignatureRef<'_>,
             _: Option<&core::ffi::CStr>,
             _: &core::ffi::CStr,
             _: crate::tree::GitTreeRef<'_>,
             _: GitCommitParents<'_>| 0,
        );
    }

    #[test]
    fn parent_pointer_view_checks_bounds_without_forming_a_slice() {
        let mut commit = crate::commit::GitCommit::zeroed();
        let pointers = [core::ptr::addr_of_mut!(commit)
            .cast::<crate::ffi::git_commit>()
            .cast_const()];
        // SAFETY: the local pointer array and opaque commit storage remain
        // live for this view, and its sole element is non-null.
        let parents = unsafe { GitCommitParents::from_raw(pointers.as_ptr(), 1) }.unwrap();
        assert_eq!(parents.len(), 1);
        assert!(!parents.is_empty());
        assert_eq!(parents.get(0).unwrap().as_ptr(), pointers[0]);
        assert!(parents.get(1).is_none());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_commit_header
    /// A layout-compatible custom commit-header descriptor.
    ///
    /// Both pointer fields borrow NUL-terminated strings owned by the caller.
    /// Libgit2 reads them synchronously while creating a commit and neither
    /// retains nor frees them.
    GitCommitHeader,
    GitCommitHeaderRef,
    GitCommitHeaderMut,
    crate::ffi::git_commit_header
);

impl<'a> GitCommitHeaderRef<'a> {
    /// Field: git_commit_header.value
    /// Borrows the header value, or returns `None` for a defensively handled
    /// null field in an incomplete C value.
    #[must_use]
    pub fn value(&self) -> Option<&'a core::ffi::CStr> {
        let header = self.as_ptr();
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference to the C-visible header.
        let value = unsafe { core::ptr::addr_of!((*header).value).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid header points to a NUL-terminated string that
            // remains live for the header handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(value) })
        }
    }

    /// Field: git_commit_header.field
    /// Borrows the header name, or returns `None` for a defensively handled
    /// null field in an incomplete C value.
    #[must_use]
    pub fn field(&self) -> Option<&'a core::ffi::CStr> {
        let header = self.as_ptr();
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference to the C-visible header.
        let field = unsafe { core::ptr::addr_of!((*header).field).read() };
        if field.is_null() {
            None
        } else {
            // SAFETY: a valid header points to a NUL-terminated string that
            // remains live for the header handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(field) })
        }
    }
}

impl GitCommitHeaderMut<'_> {
    /// Replaces the borrowed header value.
    ///
    /// # Safety
    ///
    /// `value` must remain live and NUL-terminated for every later use of the
    /// underlying `git_commit_header`, not merely for this mutable reborrow.
    pub unsafe fn set_value(&mut self, value: &core::ffi::CStr) {
        // SAFETY: the exclusive handle permits the pointer-field write; the
        // caller supplies the stored referent's unexpressible lifetime.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).value).write(value.as_ptr());
        }
    }

    /// Replaces the borrowed header name.
    ///
    /// # Safety
    ///
    /// `field` must remain live and NUL-terminated for every later use of the
    /// underlying `git_commit_header`, not merely for this mutable reborrow.
    pub unsafe fn set_field(&mut self, field: &core::ffi::CStr) {
        // SAFETY: the exclusive handle permits the pointer-field write; the
        // caller supplies the stored referent's unexpressible lifetime.
        unsafe {
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).field).write(field.as_ptr());
        }
    }
}

#[cfg(test)]
mod commit_header_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn commit_header_accessors_borrow_both_strings() {
        let mut raw = crate::ffi::git_commit_header {
            field: c"x-signature".as_ptr(),
            value: c"signed value".as_ptr(),
        };
        // SAFETY: `raw` and both static C strings remain live for the handle.
        let header = unsafe { GitCommitHeaderRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(header.field(), Some(c"x-signature"));
        assert_eq!(header.value(), Some(c"signed value"));
    }

    #[test]
    fn commit_header_mutation_replaces_borrowed_strings() {
        let mut raw = crate::ffi::git_commit_header {
            field: c"old-field".as_ptr(),
            value: c"old-value".as_ptr(),
        };
        // SAFETY: `raw` remains exclusively accessible through this handle.
        let mut header = unsafe { GitCommitHeaderMut::from_ptr(&raw mut raw) }.unwrap();
        // SAFETY: these static C strings outlive the underlying header.
        unsafe {
            header.set_field(c"new-field");
            header.set_value(c"new-value");
        }
        assert_eq!(header.as_ref().field(), Some(c"new-field"));
        assert_eq!(header.as_ref().value(), Some(c"new-value"));
    }

    #[test]
    fn commit_header_wrapper_matches_the_c_layout() {
        assert_eq!(
            size_of::<GitCommitHeader>(),
            size_of::<crate::ffi::git_commit_header>()
        );
        assert_eq!(
            align_of::<GitCommitHeader>(),
            align_of::<crate::ffi::git_commit_header>()
        );
    }
}

/// Safe callable surface for signing a commit under construction.
pub trait GitCommitSignatureCallback {
    /// Adds signature headers to `builder` for the supplied commit content.
    ///
    /// Return zero after adding a signature, `GIT_PASSTHROUGH` to leave the
    /// commit unsigned, or another libgit2 error code to abort creation.
    fn sign(
        &mut self,
        builder: crate::commit::GitCommitbuilderMut<'_>,
        repository: crate::repository::GitRepositoryMut<'_>,
        commit_content: &core::ffi::CStr,
    ) -> core::ffi::c_int;
}

impl<F> GitCommitSignatureCallback for F
where
    F: for<'a> FnMut(
        crate::commit::GitCommitbuilderMut<'a>,
        crate::repository::GitRepositoryMut<'a>,
        &'a core::ffi::CStr,
    ) -> core::ffi::c_int,
{
    fn sign(
        &mut self,
        builder: crate::commit::GitCommitbuilderMut<'_>,
        repository: crate::repository::GitRepositoryMut<'_>,
        commit_content: &core::ffi::CStr,
    ) -> core::ffi::c_int {
        self(builder, repository, commit_content)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_commit_create_ext_options
    /// Layout-compatible extended commit-creation options.
    GitCommitCreateExtOptions,
    GitCommitCreateExtOptionsRef,
    GitCommitCreateExtOptionsMut,
    crate::ffi::git_commit_create_ext_options
);

// SAFETY: extended commit options only borrow strings, headers and callback
// state. They own no resource, so disposing inline storage is a no-op.
unsafe impl CValued for GitCommitCreateExtOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitCommitCreateExtOptions {
    /// Constructs options equivalent to `GIT_COMMIT_CREATE_EXT_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(crate::ffi::GIT_COMMIT_CREATE_EXT_OPTIONS_VERSION);
        options
    }
}

impl<'a> GitCommitCreateExtOptionsRef<'a> {
    /// Field: git_commit_create_ext_options.payload
    /// Reports whether signing callback state is installed.
    #[must_use]
    pub fn has_signing_payload(&self) -> bool {
        // SAFETY: this live shared handle permits the pointer-field read.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_commit_create_ext_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_commit_create_ext_options.sign
    /// Reports whether a commit-signing callback is installed.
    #[must_use]
    pub fn has_signing_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).sign).read() }.is_some()
    }

    /// Field: git_commit_create_ext_options.extra_headers_len
    /// Returns the number of extra commit headers.
    #[must_use]
    pub fn extra_headers_len(&self) -> usize {
        // SAFETY: this live shared handle permits the scalar read.
        unsafe { addr_of!((*self.as_ptr()).extra_headers_len).read() }
    }

    /// Field: git_commit_create_ext_options.extra_headers
    /// Borrows the configured run of custom commit headers.
    #[must_use]
    pub fn extra_headers(&self) -> Option<CSlice<'a, GitCommitHeader>> {
        // SAFETY: this live shared handle permits reading the coupled pointer
        // and count fields without forming a reference to the options object.
        let (headers, len) = unsafe {
            (
                addr_of!((*self.as_ptr()).extra_headers).read(),
                addr_of!((*self.as_ptr()).extra_headers_len).read(),
            )
        };
        let headers = NonNull::new(headers.cast_mut().cast::<GitCommitHeader>())?;
        // SAFETY: a valid options value supplies `len` initialized headers at
        // this non-null pointer, all live for the options handle's borrow.
        Some(unsafe { CSlice::from_raw_parts(headers, len) })
    }

    /// Field: git_commit_create_ext_options.message_encoding
    /// Borrows the optional NUL-terminated commit-message encoding.
    #[must_use]
    pub fn message_encoding(&self) -> Option<&'a core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let value = unsafe { addr_of!((*self.as_ptr()).message_encoding).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid non-null encoding is NUL-terminated and remains
            // live for the options handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(value) })
        }
    }

    /// Field: git_commit_create_ext_options.update_ref
    /// Borrows the optional NUL-terminated reference name to update.
    #[must_use]
    pub fn update_ref(&self) -> Option<&'a core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let value = unsafe { addr_of!((*self.as_ptr()).update_ref).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid non-null reference name is NUL-terminated and
            // remains live for the options handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(value) })
        }
    }
}

impl GitCommitCreateExtOptionsMut<'_> {
    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores a borrowed run of extra commit headers.
    ///
    /// # Safety
    ///
    /// Every header and the strings it names must remain live for every later
    /// use of the underlying options value. The run must not be mutated while
    /// libgit2 reads it.
    pub unsafe fn set_borrowed_extra_headers(&mut self, headers: CSlice<'_, GitCommitHeader>) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits the coupled field writes; the
        // caller supplies the stored run's unexpressible lifetime.
        unsafe {
            addr_of_mut!((*options).extra_headers).write(headers.as_ptr().cast_const());
            addr_of_mut!((*options).extra_headers_len).write(headers.len());
        }
    }

    /// Clears the extra-header pointer and count together.
    pub fn clear_extra_headers(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes.
        unsafe {
            addr_of_mut!((*options).extra_headers).write(core::ptr::null());
            addr_of_mut!((*options).extra_headers_len).write(0);
        }
    }

    /// Stores an optional borrowed commit-message encoding.
    ///
    /// # Safety
    ///
    /// A non-null `encoding` must remain live for every later use of the
    /// underlying options value.
    pub unsafe fn set_borrowed_message_encoding(&mut self, encoding: Option<&core::ffi::CStr>) {
        let encoding = encoding.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).message_encoding).write(encoding) }
    }

    /// Stores an optional borrowed reference name to update.
    ///
    /// # Safety
    ///
    /// A non-null `update_ref` must remain live for every later use of the
    /// underlying options value.
    pub unsafe fn set_borrowed_update_ref(&mut self, update_ref: Option<&core::ffi::CStr>) {
        let update_ref = update_ref.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).update_ref).write(update_ref) }
    }

    /// Installs a typed commit-signing callback and its payload together.
    ///
    /// # Safety
    ///
    /// `callback` must remain exclusively reserved for every later use of the
    /// underlying options value. Its implementation must not unwind across
    /// the C boundary, and libgit2 must not invoke it concurrently.
    pub unsafe fn set_signing_callback<C: GitCommitSignatureCallback>(&mut self, callback: &mut C) {
        unsafe extern "C" fn trampoline<C: GitCommitSignatureCallback>(
            builder: *mut crate::ffi::git_commitbuilder,
            repository: *mut crate::ffi::git_repository,
            commit_content: *const core::ffi::c_char,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies live, exclusively callback-scoped
            // builder and repository pointers.
            let Some(builder) = (unsafe { crate::commit::GitCommitbuilderMut::from_ptr(builder) })
            else {
                return -1;
            };
            // SAFETY: as above, for the callback-scoped repository handle.
            let Some(repository) =
                (unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) })
            else {
                return -1;
            };
            if commit_content.is_null() {
                return -1;
            }
            // SAFETY: libgit2 supplies its live NUL-terminated commit buffer
            // for exactly this callback invocation.
            let content = unsafe { core::ffi::CStr::from_ptr(commit_content) };
            callback.sign(builder, repository, content)
        }

        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes; the
        // caller upholds the callback-state lifetime and invocation contract.
        unsafe {
            addr_of_mut!((*options).sign).write(Some(trampoline::<C>));
            addr_of_mut!((*options).payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the signing callback and its payload together.
    pub fn clear_signing_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes.
        unsafe {
            addr_of_mut!((*options).sign).write(None);
            addr_of_mut!((*options).payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod commit_create_ext_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitCommitCreateExtOptions>();
        assert_valued::<GitCommitCreateExtOptions>();
        assert_eq!(
            size_of::<GitCommitCreateExtOptions>(),
            size_of::<crate::ffi::git_commit_create_ext_options>()
        );
        assert_eq!(
            align_of::<GitCommitCreateExtOptions>(),
            align_of::<crate::ffi::git_commit_create_ext_options>()
        );

        let options = GitCommitCreateExtOptions::new();
        assert_eq!(
            options.as_ref().version(),
            crate::ffi::GIT_COMMIT_CREATE_EXT_OPTIONS_VERSION
        );
        assert!(options.as_ref().update_ref().is_none());
        assert!(options.as_ref().message_encoding().is_none());
        assert!(options.as_ref().extra_headers().is_none());
        assert_eq!(options.as_ref().extra_headers_len(), 0);
        assert!(!options.as_ref().has_signing_callback());
        assert!(!options.as_ref().has_signing_payload());
    }

    #[test]
    fn borrowed_strings_and_header_run_round_trip() {
        let mut raw_headers = [crate::ffi::git_commit_header {
            field: c"x-test".as_ptr(),
            value: c"value".as_ptr(),
        }];
        let ptr = NonNull::new(raw_headers.as_mut_ptr().cast::<GitCommitHeader>()).unwrap();
        // SAFETY: the initialized local header remains live and unmodified for
        // every use of the options below.
        let headers = unsafe { CSlice::from_raw_parts(ptr, raw_headers.len()) };
        let mut options = GitCommitCreateExtOptions::new();
        // SAFETY: the static strings and local header run outlive all options
        // accesses in this test.
        unsafe {
            options.as_mut().set_borrowed_update_ref(Some(c"HEAD"));
            options
                .as_mut()
                .set_borrowed_message_encoding(Some(c"UTF-8"));
            options.as_mut().set_borrowed_extra_headers(headers);
        }
        let view = options.as_ref();
        assert_eq!(view.update_ref(), Some(c"HEAD"));
        assert_eq!(view.message_encoding(), Some(c"UTF-8"));
        assert_eq!(view.extra_headers_len(), 1);
        assert_eq!(
            view.extra_headers().unwrap().get(0).unwrap().field(),
            Some(c"x-test")
        );
    }

    #[test]
    fn signing_callback_is_installed_invoked_and_cleared() {
        let mut calls = 0usize;
        let mut callback = |_: crate::commit::GitCommitbuilderMut<'_>,
                            _: crate::repository::GitRepositoryMut<'_>,
                            content: &core::ffi::CStr| {
            calls += 1;
            i32::from(content != c"content")
        };
        let mut options = GitCommitCreateExtOptions::new();
        // SAFETY: the callback remains live, does not unwind, and invocation
        // is synchronous and non-concurrent in this test.
        unsafe { options.as_mut().set_signing_callback(&mut callback) };
        assert!(options.as_ref().has_signing_callback());
        assert!(options.as_ref().has_signing_payload());

        let raw_options = options.as_ref().as_ptr();
        let mut builder = crate::commit::GitCommitbuilder::zeroed();
        let mut repository = crate::repository::GitRepository::zeroed();
        // SAFETY: the installed callback/payload pair is coupled; both opaque
        // wrapper slots and the C string remain live for this invocation.
        unsafe {
            let sign = addr_of!((*raw_options).sign).read().unwrap();
            let payload = addr_of!((*raw_options).payload).read();
            assert_eq!(
                sign(
                    addr_of_mut!(builder).cast(),
                    addr_of_mut!(repository).cast(),
                    c"content".as_ptr(),
                    payload,
                ),
                0
            );
        }
        options.as_mut().clear_signing_callback();
        assert!(!options.as_ref().has_signing_callback());
        assert!(!options.as_ref().has_signing_payload());
        drop(options);
        assert_eq!(calls, 1);
    }
}

ffibox::define_ctype!(
    /// Wraps: git_commit_create_options
    /// Layout-compatible options for the simple commit-creation APIs.
    GitCommitCreateOptions,
    GitCommitCreateOptionsRef,
    GitCommitCreateOptionsMut,
    crate::ffi::git_commit_create_options
);

// SAFETY: simple commit options only borrow signatures, strings, headers, and
// callback state. They own no resource, so disposing inline storage is a no-op.
unsafe impl CValued for GitCommitCreateOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitCommitCreateOptions {
    /// Constructs options equivalent to `GIT_COMMIT_CREATE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options
            .as_mut()
            .set_version(crate::ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION);
        options
    }
}

impl<'a> GitCommitCreateOptionsRef<'a> {
    /// Field: git_commit_create_options.payload
    /// Reports whether signing callback state is installed.
    #[must_use]
    pub fn has_signing_payload(&self) -> bool {
        // SAFETY: this live shared handle permits the pointer-field read.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_commit_create_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_commit_create_options.sign
    /// Reports whether a commit-signing callback is installed.
    #[must_use]
    pub fn has_signing_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).sign).read() }.is_some()
    }

    /// Field: git_commit_create_options.committer
    /// Borrows the optional explicit committer signature.
    #[must_use]
    pub fn committer(&self) -> Option<crate::api::types::GitSignatureRef<'a>> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let signature = unsafe { addr_of!((*self.as_ptr()).committer).read() };
        // SAFETY: a non-null field names a live signature for the options
        // handle's borrow; the constructor defensively rejects null.
        unsafe { crate::api::types::GitSignatureRef::from_ptr(signature.cast_mut()) }
    }

    /// Field: git_commit_create_options.author
    /// Borrows the optional explicit author signature.
    #[must_use]
    pub fn author(&self) -> Option<crate::api::types::GitSignatureRef<'a>> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let signature = unsafe { addr_of!((*self.as_ptr()).author).read() };
        // SAFETY: a non-null field names a live signature for the options
        // handle's borrow; the constructor defensively rejects null.
        unsafe { crate::api::types::GitSignatureRef::from_ptr(signature.cast_mut()) }
    }

    /// Field: git_commit_create_options.extra_headers_len
    /// Returns the number of extra commit headers.
    #[must_use]
    pub fn extra_headers_len(&self) -> usize {
        // SAFETY: this live shared handle permits the scalar read.
        unsafe { addr_of!((*self.as_ptr()).extra_headers_len).read() }
    }

    /// Field: git_commit_create_options.extra_headers
    /// Borrows the configured run of custom commit headers.
    #[must_use]
    pub fn extra_headers(&self) -> Option<CSlice<'a, GitCommitHeader>> {
        // SAFETY: this live shared handle permits reading the coupled pointer
        // and count fields without forming a reference to the options object.
        let (headers, len) = unsafe {
            (
                addr_of!((*self.as_ptr()).extra_headers).read(),
                addr_of!((*self.as_ptr()).extra_headers_len).read(),
            )
        };
        let headers = NonNull::new(headers.cast_mut().cast::<GitCommitHeader>())?;
        // SAFETY: a valid options value supplies `len` initialized headers at
        // this non-null pointer, all live for the options handle's borrow.
        Some(unsafe { CSlice::from_raw_parts(headers, len) })
    }

    /// Field: git_commit_create_options.message_encoding
    /// Borrows the optional NUL-terminated commit-message encoding.
    #[must_use]
    pub fn message_encoding(&self) -> Option<&'a core::ffi::CStr> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let value = unsafe { addr_of!((*self.as_ptr()).message_encoding).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid non-null encoding is NUL-terminated and remains
            // live for the options handle's borrow.
            Some(unsafe { core::ffi::CStr::from_ptr(value) })
        }
    }

    /// Field: git_commit_create_options.allow_empty_commit
    /// Returns whether creation may produce a commit without changes.
    #[must_use]
    pub fn allow_empty_commit(&self) -> bool {
        // SAFETY: bindgen's raw bitfield reader projects from this live handle
        // without forming a Rust reference to the C-visible options object.
        unsafe { crate::ffi::git_commit_create_options::allow_empty_commit_raw(self.as_ptr()) != 0 }
    }
}

impl GitCommitCreateOptionsMut<'_> {
    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Enables or disables empty commit creation.
    pub fn set_allow_empty_commit(&mut self, allow: bool) {
        // SAFETY: bindgen's raw bitfield writer projects from this exclusive
        // handle without forming a Rust reference to the C-visible object.
        unsafe {
            crate::ffi::git_commit_create_options::set_allow_empty_commit_raw(
                self.as_mut_ptr(),
                core::ffi::c_uint::from(allow),
            );
        }
    }

    /// Stores an optional borrowed author signature.
    ///
    /// # Safety
    ///
    /// A non-null `author` must remain live for every later use of the
    /// underlying options value.
    pub unsafe fn set_borrowed_author(
        &mut self,
        author: Option<crate::api::types::GitSignatureRef<'_>>,
    ) {
        let author = author.map_or(core::ptr::null(), |value| value.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).author).write(author) }
    }

    /// Stores an optional borrowed committer signature.
    ///
    /// # Safety
    ///
    /// A non-null `committer` must remain live for every later use of the
    /// underlying options value.
    pub unsafe fn set_borrowed_committer(
        &mut self,
        committer: Option<crate::api::types::GitSignatureRef<'_>>,
    ) {
        let committer = committer.map_or(core::ptr::null(), |value| value.as_ptr());
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).committer).write(committer) }
    }

    /// Stores a borrowed run of extra commit headers.
    ///
    /// # Safety
    ///
    /// Every header and the strings it names must remain live for every later
    /// use of the underlying options value. The run must not be mutated while
    /// libgit2 reads it.
    pub unsafe fn set_borrowed_extra_headers(&mut self, headers: CSlice<'_, GitCommitHeader>) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits the coupled field writes; the
        // caller supplies the stored run's unexpressible lifetime.
        unsafe {
            addr_of_mut!((*options).extra_headers).write(headers.as_ptr().cast_const());
            addr_of_mut!((*options).extra_headers_len).write(headers.len());
        }
    }

    /// Clears the extra-header pointer and count together.
    pub fn clear_extra_headers(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes.
        unsafe {
            addr_of_mut!((*options).extra_headers).write(core::ptr::null());
            addr_of_mut!((*options).extra_headers_len).write(0);
        }
    }

    /// Stores an optional borrowed commit-message encoding.
    ///
    /// # Safety
    ///
    /// A non-null `encoding` must remain live for every later use of the
    /// underlying options value.
    pub unsafe fn set_borrowed_message_encoding(&mut self, encoding: Option<&core::ffi::CStr>) {
        let encoding = encoding.map_or(core::ptr::null(), core::ffi::CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).message_encoding).write(encoding) }
    }

    /// Installs a typed commit-signing callback and its payload together.
    ///
    /// # Safety
    ///
    /// `callback` must remain exclusively reserved for every later use of the
    /// underlying options value. Its implementation must not unwind across
    /// the C boundary, and libgit2 must not invoke it concurrently.
    pub unsafe fn set_signing_callback<C: GitCommitSignatureCallback>(&mut self, callback: &mut C) {
        unsafe extern "C" fn trampoline<C: GitCommitSignatureCallback>(
            builder: *mut crate::ffi::git_commitbuilder,
            repository: *mut crate::ffi::git_repository,
            commit_content: *const core::ffi::c_char,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: installation stores a live exclusive `C` in `payload`.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            // SAFETY: libgit2 supplies live, exclusively callback-scoped
            // builder and repository pointers.
            let Some(builder) = (unsafe { crate::commit::GitCommitbuilderMut::from_ptr(builder) })
            else {
                return -1;
            };
            // SAFETY: as above, for the callback-scoped repository handle.
            let Some(repository) =
                (unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) })
            else {
                return -1;
            };
            if commit_content.is_null() {
                return -1;
            }
            // SAFETY: libgit2 supplies its live NUL-terminated commit buffer
            // for exactly this callback invocation.
            let content = unsafe { core::ffi::CStr::from_ptr(commit_content) };
            callback.sign(builder, repository, content)
        }

        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes; the
        // caller upholds the callback-state lifetime and invocation contract.
        unsafe {
            addr_of_mut!((*options).sign).write(Some(trampoline::<C>));
            addr_of_mut!((*options).payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the signing callback and its payload together.
    pub fn clear_signing_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits both coupled field writes.
        unsafe {
            addr_of_mut!((*options).sign).write(None);
            addr_of_mut!((*options).payload).write(core::ptr::null_mut());
        }
    }
}

/// Borrowed, indexed view over the commit pointers owned by a commit array.
#[derive(Clone, Copy)]
pub struct GitCommitArrayItems<'a> {
    ptr: NonNull<*mut crate::ffi::git_commit>,
    len: usize,
    _borrow: PhantomData<crate::commit::GitCommitRef<'a>>,
}

impl<'a> GitCommitArrayItems<'a> {
    /// Returns the number of commits in the view.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether the view contains no commits.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one commit by position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<crate::commit::GitCommitRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: the array wrapper guarantees a readable `len`-element
        // pointer allocation, and the bounds check selects one element.
        let commit = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: every valid element is an owned live commit reference;
        // malformed null elements are defensively rejected.
        unsafe { crate::commit::GitCommitRef::from_ptr(commit) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_commitarray
    /// A by-value header owning an array of independently owned commits.
    GitCommitArray,
    GitCommitArrayRef,
    GitCommitArrayMut,
    crate::ffi::git_commitarray
);

// SAFETY: `git_commitarray_dispose` releases each owned commit reference and
// the pointer allocation, but not the by-value header itself. `GitCommitArray`
// is transparent over that initialized header.
unsafe impl CValued for GitCommitArray {
    unsafe fn c_dispose(this: NonNull<Self>) {
        // SAFETY: the `CValued` contract supplies a live initialized header;
        // the C disposer consumes only its owned fields and zeroes the header.
        unsafe { crate::ffi::git_commitarray_dispose(this.as_ptr().cast()) }
    }
}

impl GitCommitArray {
    /// Constructs an empty, disposable commit array for an out-parameter.
    #[must_use]
    pub fn new() -> CVal<Self> {
        CVal::new(Self::zeroed())
    }
}

impl<'a> GitCommitArrayRef<'a> {
    /// Field: git_commitarray.count
    /// Returns the number of owned commit references.
    #[must_use]
    pub fn count(&self) -> usize {
        // SAFETY: this live shared handle permits the scalar read.
        unsafe { addr_of!((*self.as_ptr()).count).read() }
    }

    /// Field: git_commitarray.commits
    /// Borrows the owned commit references without transferring them.
    #[must_use]
    pub fn commits(&self) -> Option<GitCommitArrayItems<'a>> {
        // SAFETY: this live shared handle permits reading the coupled pointer
        // and count without forming a reference to the C-visible header.
        let (commits, len) = unsafe {
            (
                addr_of!((*self.as_ptr()).commits).read(),
                addr_of!((*self.as_ptr()).count).read(),
            )
        };
        let ptr = if commits.is_null() && len == 0 {
            NonNull::dangling()
        } else {
            NonNull::new(commits.cast_mut())?
        };
        Some(GitCommitArrayItems {
            ptr,
            len,
            _borrow: PhantomData,
        })
    }
}

#[cfg(test)]
mod commit_create_options_and_array_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn simple_options_preserve_layout_and_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitCommitCreateOptions>();
        assert_valued::<GitCommitCreateOptions>();
        assert_eq!(
            size_of::<GitCommitCreateOptions>(),
            size_of::<crate::ffi::git_commit_create_options>()
        );
        assert_eq!(
            align_of::<GitCommitCreateOptions>(),
            align_of::<crate::ffi::git_commit_create_options>()
        );

        let options = GitCommitCreateOptions::new();
        let view = options.as_ref();
        assert_eq!(
            view.version(),
            crate::ffi::GIT_COMMIT_CREATE_OPTIONS_VERSION
        );
        assert!(!view.allow_empty_commit());
        assert!(view.author().is_none());
        assert!(view.committer().is_none());
        assert!(view.message_encoding().is_none());
        assert!(view.extra_headers().is_none());
        assert_eq!(view.extra_headers_len(), 0);
        assert!(!view.has_signing_callback());
        assert!(!view.has_signing_payload());
    }

    #[test]
    fn simple_options_borrow_fields_and_invoke_signer() {
        let mut signature = crate::api::types::GitSignature::zeroed();
        // SAFETY: the local signature storage remains live for every options
        // access below; no signature field is inspected.
        let signature = unsafe {
            crate::api::types::GitSignatureRef::from_ptr(
                addr_of_mut!(signature).cast::<crate::ffi::git_signature>(),
            )
        }
        .unwrap();
        let mut raw_headers = [crate::ffi::git_commit_header {
            field: c"x-test".as_ptr(),
            value: c"value".as_ptr(),
        }];
        let headers_ptr = NonNull::new(raw_headers.as_mut_ptr().cast::<GitCommitHeader>()).unwrap();
        // SAFETY: the initialized local header remains live and unmodified for
        // every use of the options below.
        let headers = unsafe { CSlice::from_raw_parts(headers_ptr, raw_headers.len()) };
        let mut calls = 0usize;
        let mut callback = |_: crate::commit::GitCommitbuilderMut<'_>,
                            _: crate::repository::GitRepositoryMut<'_>,
                            content: &core::ffi::CStr| {
            calls += 1;
            i32::from(content != c"content")
        };
        let mut options = GitCommitCreateOptions::new();
        // SAFETY: all borrowed locals remain live, the callback does not
        // unwind, and its manual invocation below is synchronous.
        unsafe {
            options.as_mut().set_borrowed_author(Some(signature));
            options.as_mut().set_borrowed_committer(Some(signature));
            options
                .as_mut()
                .set_borrowed_message_encoding(Some(c"UTF-8"));
            options.as_mut().set_borrowed_extra_headers(headers);
            options.as_mut().set_signing_callback(&mut callback);
        }
        options.as_mut().set_allow_empty_commit(true);

        let view = options.as_ref();
        assert_eq!(view.author().unwrap().as_ptr(), signature.as_ptr());
        assert_eq!(view.committer().unwrap().as_ptr(), signature.as_ptr());
        assert_eq!(view.message_encoding(), Some(c"UTF-8"));
        assert_eq!(view.extra_headers_len(), 1);
        assert_eq!(
            view.extra_headers().unwrap().get(0).unwrap().field(),
            Some(c"x-test")
        );
        assert!(view.allow_empty_commit());

        let raw_options = view.as_ptr();
        let mut builder = crate::commit::GitCommitbuilder::zeroed();
        let mut repository = crate::repository::GitRepository::zeroed();
        // SAFETY: the installed callback/payload pair is coupled; both opaque
        // wrapper slots and the C string remain live for this invocation.
        unsafe {
            let sign = addr_of!((*raw_options).sign).read().unwrap();
            let payload = addr_of!((*raw_options).payload).read();
            assert_eq!(
                sign(
                    addr_of_mut!(builder).cast(),
                    addr_of_mut!(repository).cast(),
                    c"content".as_ptr(),
                    payload,
                ),
                0
            );
        }
        options.as_mut().clear_signing_callback();
        drop(options);
        assert_eq!(calls, 1);
    }

    #[test]
    fn commit_array_preserves_layout_and_borrows_elements() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitCommitArray>();
        assert_valued::<GitCommitArray>();
        assert_eq!(
            size_of::<GitCommitArray>(),
            size_of::<crate::ffi::git_commitarray>()
        );
        assert_eq!(
            align_of::<GitCommitArray>(),
            align_of::<crate::ffi::git_commitarray>()
        );

        let mut commit = crate::commit::GitCommit::zeroed();
        let pointers = [addr_of_mut!(commit).cast::<crate::ffi::git_commit>()];
        let mut raw = crate::ffi::git_commitarray {
            commits: pointers.as_ptr(),
            count: pointers.len(),
        };
        // SAFETY: `raw`, the pointer array, and the opaque commit storage all
        // remain live for this borrowed view; its sole element is non-null.
        let array = unsafe { GitCommitArrayRef::from_ptr(&raw mut raw) }.unwrap();
        let commits = array.commits().unwrap();
        assert_eq!(array.count(), 1);
        assert_eq!(commits.len(), 1);
        assert!(!commits.is_empty());
        assert_eq!(commits.get(0).unwrap().as_ptr(), pointers[0].cast_const());
        assert!(commits.get(1).is_none());

        let empty = GitCommitArray::new();
        assert_eq!(empty.as_ref().count(), 0);
        assert!(empty.as_ref().commits().unwrap().is_empty());
    }
}
