//! Safe wrappers for libgit2 rebase APIs.

use core::ffi::{CStr, c_char, c_int, c_void};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::buffer::GitBufMut;
use crate::api::checkout::{GitCheckoutOptionsMut, GitCheckoutOptionsRef};
use crate::api::commit::{GitCommitCreateCallback, GitCommitParents};
use crate::api::merge::{GitMergeOptionsMut, GitMergeOptionsRef};
use crate::ffi;

/// Safe callable surface for the deprecated rebase-signing callback.
pub trait GitRebaseSigningCallback {
    /// Produces a signature and optional signature-field name for commit text.
    fn sign(
        &mut self,
        signature: &mut GitBufMut<'_>,
        signature_field: &mut GitBufMut<'_>,
        commit_content: &CStr,
    ) -> c_int;
}

impl<F> GitRebaseSigningCallback for F
where
    F: FnMut(&mut GitBufMut<'_>, &mut GitBufMut<'_>, &CStr) -> c_int,
{
    fn sign(
        &mut self,
        signature: &mut GitBufMut<'_>,
        signature_field: &mut GitBufMut<'_>,
        commit_content: &CStr,
    ) -> c_int {
        self(signature, signature_field, commit_content)
    }
}

/// Wraps: git_rebase_options
/// Layout-compatible rebase options borrowing strings, nested option data,
/// and callback state for `'data`.
///
/// `'data` is invariant. [`GitRebaseOptionsMut::set_rewrite_notes_ref`]
/// stores a `&'data` string and the two callback setters store a `&'data mut`
/// receiver, while the getters hand the string back out and libgit2 keeps the
/// payload for the life of the rebase. A covariant `'data` would let safe
/// code shrink the parameter on the exclusive handle, install a shorter-lived
/// string or receiver, and use it through a handle still typed at the longer
/// lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::rebase::GitRebaseOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitRebaseOptionsMut<'object, 'static>,
/// ) -> GitRebaseOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitRebaseOptions<'data> {
    inner: CType<ffi::git_rebase_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: core::marker::PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitRebaseOptions`].
#[repr(transparent)]
pub struct GitRebaseOptionsRef<'object, 'data>(CPtr<'object, GitRebaseOptions<'data>>);

impl Clone for GitRebaseOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for GitRebaseOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitRebaseOptions`].
#[repr(transparent)]
pub struct GitRebaseOptionsMut<'object, 'data>(GitRebaseOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitRebaseOptions<'data> {
    type C = ffi::git_rebase_options;
    type Ref<'object>
        = GitRebaseOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitRebaseOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitRebaseOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitRebaseOptionsMut(GitRebaseOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: this public options record only borrows its pointer fields. Neither
// it nor its nested options owns a resource that an inline disposer must free.
unsafe impl CValued for GitRebaseOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitRebaseOptions<'data> {
    /// Constructs options equivalent to `GIT_REBASE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: all fields in the C options record admit the all-zero bit
        // pattern. Required versions and documented nonzero defaults are set
        // through exclusive field projections before return.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: core::marker::PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_REBASE_OPTIONS_VERSION);
            view.merge_options_mut()
                .set_version(ffi::GIT_MERGE_OPTIONS_VERSION);
            view.merge_options_mut()
                .set_flags(crate::api::merge::GitMergeFlags::FIND_RENAMES);
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
        }
        options
    }
}

impl<'object, 'data> GitRebaseOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// `ptr` must identify initialized options that live for `'object`.
    /// Every configured string, nested borrow, and callback payload must
    /// remain valid for `'data`, which must outlive `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_rebase_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitRebaseOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_rebase_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_rebase_options.payload
    /// Reports whether callback state is installed.
    #[must_use]
    pub fn has_callback_payload(&self) -> bool {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        !unsafe { addr_of!((*self.as_ptr()).payload).read() }.is_null()
    }

    /// Field: git_rebase_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_rebase_options.reserved
    /// Returns the reserved hard-deprecation slot for this binding mode.
    ///
    /// This workspace is configured with `DEPRECATE_HARD=OFF`, so the C ABI
    /// exposes `signing_cb` in that union position and has no reserved value.
    #[must_use]
    pub const fn reserved(&self) -> Option<NonNull<c_void>> {
        None
    }

    /// Field: git_rebase_options.commit_create_cb
    /// Reports whether a custom commit-creation callback is installed.
    #[must_use]
    pub fn has_commit_create_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).commit_create_cb).read() }.is_some()
    }

    /// Field: git_rebase_options.signing_cb
    /// Reports whether the deprecated signing callback is installed.
    #[must_use]
    pub fn has_signing_callback(&self) -> bool {
        // SAFETY: this live shared handle permits the callback-slot read.
        unsafe { addr_of!((*self.as_ptr()).signing_cb).read() }.is_some()
    }

    /// Field: git_rebase_options.inmemory
    /// Returns whether the rebase runs without changing repository state.
    #[must_use]
    pub fn inmemory(&self) -> bool {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).inmemory).read() != 0 }
    }

    /// Field: git_rebase_options.quiet
    /// Returns whether other clients should provide a quiet experience.
    #[must_use]
    pub fn quiet(&self) -> bool {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).quiet).read() != 0 }
    }

    /// Field: git_rebase_options.checkout_options
    /// Borrows the inline checkout options.
    #[must_use]
    pub fn checkout_options(&self) -> GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).checkout_options).cast_mut() };
        // SAFETY: the projected field lives for this outer borrow and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitCheckoutOptionsRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Field: git_rebase_options.merge_options
    /// Borrows the inline merge options.
    #[must_use]
    pub fn merge_options(&self) -> GitMergeOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // without forming a reference to C-visible storage.
        let ptr = unsafe { addr_of!((*self.as_ptr()).merge_options).cast_mut() };
        // SAFETY: the projected field lives for this outer borrow and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitMergeOptionsRef::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Field: git_rebase_options.rewrite_notes_ref
    /// Borrows the optional NUL-terminated notes-reference name.
    #[must_use]
    pub fn rewrite_notes_ref(&self) -> Option<&'object CStr> {
        // SAFETY: this live shared handle permits the pointer raw-place read.
        let reference = unsafe { addr_of!((*self.as_ptr()).rewrite_notes_ref).read() };
        if reference.is_null() {
            None
        } else {
            // SAFETY: the wrapper contract keeps the configured NUL string
            // valid for at least this options borrow.
            Some(unsafe { CStr::from_ptr(reference) })
        }
    }
}

impl<'object, 'data> GitRebaseOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    ///
    /// The shared-handle requirements apply, and no other access path may use
    /// the options value for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_rebase_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitRebaseOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_rebase_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitRebaseOptionsRef<'_, 'data> {
        GitRebaseOptionsRef(self.0.0)
    }

    /// Sets the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this live exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects or disables an in-memory rebase.
    pub fn set_inmemory(&mut self, inmemory: bool) {
        // SAFETY: this live exclusive handle permits the scalar write, and C
        // interprets this field as a boolean integer.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).inmemory).write(c_int::from(inmemory)) }
    }

    /// Selects or disables quiet interoperability behavior.
    pub fn set_quiet(&mut self, quiet: bool) {
        // SAFETY: this live exclusive handle permits the scalar write, and C
        // interprets this field as a boolean integer.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).quiet).write(c_int::from(quiet)) }
    }

    /// Stores an optional borrowed notes-reference name.
    pub fn set_rewrite_notes_ref(&mut self, reference: Option<&'data CStr>) {
        let reference = reference.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this live exclusive handle permits the pointer write, and
        // the wrapper lifetime keeps a non-null NUL string live.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).rewrite_notes_ref).write(reference) }
    }

    /// Installs a commit-creation callback and its borrowed state.
    ///
    /// The callback remains borrowed after libgit2 copies these options into
    /// a rebase; a safe rebase constructor must therefore keep `'data` alive
    /// for the resulting rebase handle.
    pub fn set_commit_create_callback<C>(&mut self, callback: &'data mut C)
    where
        C: GitCommitCreateCallback,
    {
        let ptr = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three writes. The generic
        // trampoline expects exactly the `C` pointer installed as payload,
        // and clearing signing_cb preserves libgit2's one-callback protocol.
        unsafe {
            addr_of_mut!((*ptr).payload).write(core::ptr::from_mut(callback).cast());
            addr_of_mut!((*ptr).commit_create_cb).write(Some(commit_create_trampoline::<C>));
            addr_of_mut!((*ptr).signing_cb).write(None);
        }
    }

    /// Installs the deprecated signing callback and its borrowed state.
    ///
    /// The callback remains borrowed after libgit2 copies these options into
    /// a rebase; a safe rebase constructor must therefore keep `'data` alive
    /// for the resulting rebase handle.
    pub fn set_signing_callback<C>(&mut self, callback: &'data mut C)
    where
        C: GitRebaseSigningCallback,
    {
        let ptr = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three writes. The generic
        // trampoline expects exactly the `C` pointer installed as payload,
        // and clearing commit_create_cb preserves libgit2's precedence rule.
        unsafe {
            addr_of_mut!((*ptr).payload).write(core::ptr::from_mut(callback).cast());
            addr_of_mut!((*ptr).signing_cb).write(Some(signing_trampoline::<C>));
            addr_of_mut!((*ptr).commit_create_cb).write(None);
        }
    }

    /// Clears both callback slots and their shared payload.
    pub fn clear_callbacks(&mut self) {
        let ptr = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits all three pointer writes.
        unsafe {
            addr_of_mut!((*ptr).payload).write(core::ptr::null_mut());
            addr_of_mut!((*ptr).commit_create_cb).write(None);
            addr_of_mut!((*ptr).signing_cb).write(None);
        }
    }

    /// Exclusively borrows the inline checkout options.
    #[must_use]
    pub fn checkout_options_mut(&mut self) -> GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // under this exclusive outer reborrow.
        let ptr = unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_options) };
        // SAFETY: the projected field is exclusively accessible and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitCheckoutOptionsMut::from_ptr(ptr) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the inline merge options.
    #[must_use]
    pub fn merge_options_mut(&mut self) -> GitMergeOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection reaches the initialized inline field
        // under this exclusive outer reborrow.
        let ptr = unsafe { addr_of_mut!((*self.as_mut_ptr()).merge_options) };
        // SAFETY: the projected field is exclusively accessible and shares
        // the outer record's borrowed-data lifetime.
        unsafe { GitMergeOptionsMut::from_ptr(ptr) }.expect("an inline field is non-null")
    }
}

unsafe extern "C" fn commit_create_trampoline<C>(
    out: *mut ffi::git_oid,
    author: *const ffi::git_signature,
    committer: *const ffi::git_signature,
    message_encoding: *const c_char,
    message: *const c_char,
    tree: *const ffi::git_tree,
    parent_count: usize,
    parents: *mut *const ffi::git_commit,
    payload: *mut c_void,
) -> c_int
where
    C: GitCommitCreateCallback,
{
    let invoke = || {
        // SAFETY: libgit2 invokes the installed callback with non-null live
        // output, signature, message, tree, and payload pointers. The optional
        // encoding is either null or a live NUL string. The parent pointer run
        // contains `parent_count` non-null borrowed commits for this call.
        unsafe {
            let mut out = crate::oid::OidMut::from_ptr(out).expect("libgit2 supplies an output ID");
            let author = crate::api::types::GitSignatureRef::from_ptr(author.cast_mut())
                .expect("libgit2 supplies an author");
            let committer = crate::api::types::GitSignatureRef::from_ptr(committer.cast_mut())
                .expect("libgit2 supplies a committer");
            let encoding = if message_encoding.is_null() {
                None
            } else {
                Some(CStr::from_ptr(message_encoding))
            };
            let message = CStr::from_ptr(message);
            let tree = crate::tree::GitTreeRef::from_ptr(tree.cast_mut())
                .expect("libgit2 supplies a tree");
            let parents = GitCommitParents::from_raw(parents.cast_const(), parent_count)
                .expect("libgit2 supplies the parent pointer run");
            (&mut *payload.cast::<C>()).create(
                &mut out, author, committer, encoding, message, tree, parents,
            )
        }
    };

    std::panic::catch_unwind(std::panic::AssertUnwindSafe(invoke))
        .unwrap_or(ffi::git_error_code_GIT_EUSER)
}

unsafe extern "C" fn signing_trampoline<C>(
    signature: *mut ffi::git_buf,
    signature_field: *mut ffi::git_buf,
    commit_content: *const c_char,
    payload: *mut c_void,
) -> c_int
where
    C: GitRebaseSigningCallback,
{
    let invoke = || {
        // SAFETY: libgit2 invokes the installed callback with two live,
        // disjoint writable buffer headers, NUL-terminated commit text, and
        // the exact callback state pointer installed by the safe setter.
        unsafe {
            let mut signature =
                GitBufMut::from_ptr(signature).expect("libgit2 supplies a signature buffer");
            let mut signature_field = GitBufMut::from_ptr(signature_field)
                .expect("libgit2 supplies a signature-field buffer");
            let content = CStr::from_ptr(commit_content);
            (&mut *payload.cast::<C>()).sign(&mut signature, &mut signature_field, content)
        }
    };

    std::panic::catch_unwind(std::panic::AssertUnwindSafe(invoke))
        .unwrap_or(ffi::git_error_code_GIT_EUSER)
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn options_preserve_layout_and_public_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}
        assert_cell::<GitRebaseOptions<'static>>();
        assert_valued::<GitRebaseOptions<'static>>();
        assert_eq!(
            size_of::<GitRebaseOptions<'static>>(),
            size_of::<ffi::git_rebase_options>()
        );
        assert_eq!(
            align_of::<GitRebaseOptions<'static>>(),
            align_of::<ffi::git_rebase_options>()
        );
        assert_eq!(
            size_of::<GitRebaseOptionsRef<'static, 'static>>(),
            size_of::<*const ffi::git_rebase_options>()
        );

        let options = GitRebaseOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_REBASE_OPTIONS_VERSION);
        assert!(!view.quiet());
        assert!(!view.inmemory());
        assert!(view.rewrite_notes_ref().is_none());
        assert_eq!(
            view.merge_options().flags(),
            Ok(crate::api::merge::GitMergeFlags::FIND_RENAMES)
        );
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert!(!view.has_commit_create_callback());
        assert!(!view.has_signing_callback());
        assert!(!view.has_callback_payload());
        assert!(view.reserved().is_none());
    }

    #[test]
    fn scalar_string_and_nested_fields_round_trip() {
        let mut options = GitRebaseOptions::new();
        {
            let mut view = options.as_mut();
            view.set_quiet(true);
            view.set_inmemory(true);
            view.set_rewrite_notes_ref(Some(c"refs/notes/rebased"));
            view.merge_options_mut().set_target_limit(99);
            view.checkout_options_mut().set_file_mode(0o100644);
        }

        let view = options.as_ref();
        assert!(view.quiet());
        assert!(view.inmemory());
        assert_eq!(view.rewrite_notes_ref(), Some(c"refs/notes/rebased"));
        assert_eq!(view.merge_options().target_limit(), 99);
        assert_eq!(view.checkout_options().file_mode(), 0o100644);
    }

    #[test]
    fn signing_callback_is_typed_invoked_and_cleared() {
        let mut calls = 0usize;
        {
            // The receiver is borrowed for the options' whole `'data`, which
            // the invariant marker pins, so `calls` is readable only once the
            // options and the closure are both gone.
            let mut callback = |_: &mut GitBufMut<'_>, _: &mut GitBufMut<'_>, content: &CStr| {
                assert_eq!(content, c"tree deadbeef\n");
                calls += 1;
                17
            };
            let mut options = GitRebaseOptions::new();
            {
                let mut view = options.as_mut();
                view.set_signing_callback(&mut callback);
            }

            let raw = options.as_ref().as_ptr();
            // SAFETY: the options setter installed this callback with its
            // matching live payload, and both local buffer headers are valid
            // and disjoint.
            let result = unsafe {
                let mut signature: ffi::git_buf = core::mem::zeroed();
                let mut field: ffi::git_buf = core::mem::zeroed();
                addr_of!((*raw).signing_cb)
                    .read()
                    .expect("callback installed")(
                    addr_of_mut!(signature),
                    addr_of_mut!(field),
                    c"tree deadbeef\n".as_ptr(),
                    addr_of!((*raw).payload).read(),
                )
            };
            assert_eq!(result, 17);

            options.as_mut().clear_callbacks();
            let view = options.as_ref();
            assert!(!view.has_signing_callback());
            assert!(!view.has_callback_payload());
        }
        assert_eq!(calls, 1);
    }
}
