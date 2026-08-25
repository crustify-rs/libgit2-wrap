//! Safe wrappers for libgit2 clone APIs.

use core::ffi::{CStr, c_int, c_uint, c_void};
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CCell, CPtr, CType, CVal, CValued};

use crate::api::checkout::{GitCheckoutOptionsMut, GitCheckoutOptionsRef};
use crate::api::remote::{GitFetchOptionsMut, GitFetchOptionsRef};
use crate::clone::GitCloneLocal;
use crate::ffi;
use crate::remote::GitRemoteWithRepository;
use crate::repository::{GitRepositoryMut, GitRepositoryOwned};

/// Safe implementation of a repository-creation callback used by clone.
pub trait GitRepositoryCreateCallback {
    /// Creates and returns the repository at `path`.
    fn create(&mut self, path: &CStr, bare: bool) -> Result<GitRepositoryOwned, i32>;
}

impl<F> GitRepositoryCreateCallback for F
where
    F: FnMut(&CStr, bool) -> Result<GitRepositoryOwned, i32>,
{
    fn create(&mut self, path: &CStr, bare: bool) -> Result<GitRepositoryOwned, i32> {
        self(path, bare)
    }
}

/// Safe implementation of a remote-creation callback used by clone.
pub trait GitRemoteCreateCallback {
    /// Creates and returns the remote named `name` for `repository`.
    fn create<'repo>(
        &mut self,
        repository: &'repo mut GitRepositoryMut<'_>,
        name: &CStr,
        url: &CStr,
    ) -> Result<GitRemoteWithRepository<'repo>, i32>;
}

impl<F> GitRemoteCreateCallback for F
where
    F: for<'repo> FnMut(
        &'repo mut GitRepositoryMut<'_>,
        &CStr,
        &CStr,
    ) -> Result<GitRemoteWithRepository<'repo>, i32>,
{
    fn create<'repo>(
        &mut self,
        repository: &'repo mut GitRepositoryMut<'_>,
        name: &CStr,
        url: &CStr,
    ) -> Result<GitRemoteWithRepository<'repo>, i32> {
        self(repository, name, url)
    }
}

/// Wraps: git_clone_options
/// Layout-compatible clone options borrowing all nested strings, objects and
/// callback state for `'data`.
///
/// `'data` is invariant. [`GitCloneOptionsMut::set_checkout_branch`] stores a
/// `&'data` branch name in the C struct and
/// [`GitCloneOptionsRef::checkout_branch`] hands it back out; the callback
/// setters store a `&'data mut` receiver that every clone invocation
/// dereferences; and the embedded checkout and fetch options reached through
/// [`GitCloneOptionsMut::checkout_options_mut`] and
/// [`GitCloneOptionsMut::fetch_options_mut`] inherit `'data` and store their
/// own referents the same way. A covariant `'data` would let safe code shrink
/// the parameter on the exclusive handle, install a shorter-lived branch name,
/// proxy URL or callback receiver, and then read the released storage back
/// through a handle still typed at the longer lifetime.
///
/// Shrinking `'data` is therefore rejected:
///
/// ```compile_fail
/// use libgit2::api::clone::GitCloneOptionsMut;
///
/// fn shrink<'object, 'short>(
///     options: GitCloneOptionsMut<'object, 'static>,
/// ) -> GitCloneOptionsMut<'object, 'short> {
///     options
/// }
/// ```
#[repr(transparent)]
pub struct GitCloneOptions<'data> {
    inner: CType<ffi::git_clone_options>,
    // The canonical invariance marker: a function type is contravariant in
    // its argument and covariant in its result, so naming `'data` in both
    // positions pins it. `&'data ()` and `&'data mut ()` are both covariant
    // and would not. The `fn` pointer keeps the auto traits unchanged.
    _data: PhantomData<fn(&'data ()) -> &'data ()>,
}

/// Shared borrow of [`GitCloneOptions`].
#[repr(transparent)]
pub struct GitCloneOptionsRef<'object, 'data>(CPtr<'object, GitCloneOptions<'data>>);

impl Clone for GitCloneOptionsRef<'_, '_> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for GitCloneOptionsRef<'_, '_> {}

/// Exclusive borrow of [`GitCloneOptions`].
#[repr(transparent)]
pub struct GitCloneOptionsMut<'object, 'data>(GitCloneOptionsRef<'object, 'data>);

// SAFETY: the layout type is transparent over the matching bindgen struct;
// both handles are pointer-sized and access C-visible storage only through
// raw-place projections. The shared handle exposes no writes.
unsafe impl<'data> CCell for GitCloneOptions<'data> {
    type C = ffi::git_clone_options;
    type Ref<'object>
        = GitCloneOptionsRef<'object, 'data>
    where
        Self: 'object;
    type Mut<'object>
        = GitCloneOptionsMut<'object, 'data>
    where
        Self: 'object;

    unsafe fn ref_from_raw<'object>(ptr: NonNull<Self>) -> Self::Ref<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller guarantees that `ptr` is a live shared object.
        GitCloneOptionsRef(unsafe { CPtr::new(ptr) })
    }

    unsafe fn mut_from_raw<'object>(ptr: NonNull<Self>) -> Self::Mut<'object>
    where
        Self: 'object,
    {
        // SAFETY: the caller additionally guarantees exclusive access.
        GitCloneOptionsMut(GitCloneOptionsRef(unsafe { CPtr::new(ptr) }))
    }
}

// SAFETY: clone options own no resource. Every pointer in the header or its
// inline option records is borrowed for the wrapper's data lifetime.
unsafe impl CValued for GitCloneOptions<'_> {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl<'data> GitCloneOptions<'data> {
    /// Constructs options equivalent to `GIT_CLONE_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // SAFETY: every raw field admits zero. Required versions and nonzero
        // nested defaults are installed before the initialized value escapes.
        let inner = unsafe { CType::zeroed() };
        let mut options = CVal::new(Self {
            inner,
            _data: PhantomData,
        });
        {
            let mut view = options.as_mut();
            view.set_version(ffi::GIT_CLONE_OPTIONS_VERSION);
            view.checkout_options_mut()
                .set_version(ffi::GIT_CHECKOUT_OPTIONS_VERSION);
            initialize_fetch_options(view.fetch_options_mut());
        }
        options
    }
}

fn initialize_fetch_options(mut options: GitFetchOptionsMut<'_, '_>) {
    options.set_version(ffi::GIT_FETCH_OPTIONS_VERSION as c_int);
    options.set_update_flags(1);
    options
        .callbacks_mut()
        .set_version(ffi::GIT_REMOTE_CALLBACKS_VERSION);
    options
        .proxy_options_mut()
        .set_version(ffi::GIT_PROXY_OPTIONS_VERSION);
}

impl<'object, 'data> GitCloneOptionsRef<'object, 'data> {
    /// Borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    /// `ptr` must identify initialized options live for `'object`. Every
    /// nested borrow and callback payload must remain valid for `'data`, which
    /// must outlive `'object`; callback payload types must match their slots.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_clone_options) -> Option<Self> {
        NonNull::new(ptr.cast::<GitCloneOptions<'data>>()).map(|ptr| {
            // SAFETY: the caller supplies the required live shared object.
            Self(unsafe { CPtr::new(ptr) })
        })
    }

    /// Returns the C pointer for read-only FFI calls.
    #[must_use]
    pub fn as_ptr(&self) -> *const ffi::git_clone_options {
        self.0.as_non_null().as_ptr().cast()
    }

    /// Field: git_clone_options.version
    /// Returns the clone-options ABI version.
    #[must_use]
    pub fn version(&self) -> c_uint {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_clone_options.checkout_opts
    /// Borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options(&self) -> GitCheckoutOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { addr_of!((*self.as_ptr()).checkout_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitCheckoutOptionsRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_clone_options.fetch_opts
    /// Borrows the embedded fetch options.
    #[must_use]
    pub fn fetch_options(&self) -> GitFetchOptionsRef<'object, 'data> {
        // SAFETY: raw-place projection locates the initialized inline field.
        let field = unsafe { addr_of!((*self.as_ptr()).fetch_opts).cast_mut() };
        // SAFETY: the field lives for the enclosing options borrow and retains
        // the enclosing data lifetime.
        unsafe { GitFetchOptionsRef::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Field: git_clone_options.bare
    /// Returns whether clone creates a bare repository.
    #[must_use]
    pub fn bare(&self) -> bool {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        unsafe { addr_of!((*self.as_ptr()).bare).read() != 0 }
    }

    /// Field: git_clone_options.local
    /// Returns the local-clone policy, rejecting unknown C values.
    pub fn local(&self) -> Result<GitCloneLocal, ffi::git_clone_local_t> {
        // SAFETY: this live shared handle permits the scalar raw-place read.
        let value = unsafe { addr_of!((*self.as_ptr()).local).read() };
        GitCloneLocal::try_from(value)
    }

    /// Field: git_clone_options.checkout_branch
    /// Borrows the optional branch name.
    #[must_use]
    pub fn checkout_branch(&self) -> Option<&'object CStr> {
        // SAFETY: raw-place projection copies the initialized string pointer.
        let branch = unsafe { addr_of!((*self.as_ptr()).checkout_branch).read() };
        if branch.is_null() {
            None
        } else {
            // SAFETY: valid clone options keep this NUL-terminated string live
            // and immutable for the enclosing options borrow.
            Some(unsafe { CStr::from_ptr(branch) })
        }
    }

    /// Field: git_clone_options.repository_cb
    /// Reports whether a repository-creation callback is installed.
    #[must_use]
    pub fn has_repository_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { addr_of!((*self.as_ptr()).repository_cb).read() }.is_some()
    }

    /// Field: git_clone_options.repository_cb_payload
    /// Reports whether repository-callback state is installed.
    #[must_use]
    pub fn has_repository_callback_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { addr_of!((*self.as_ptr()).repository_cb_payload).read() }.is_null()
    }

    /// Field: git_clone_options.remote_cb
    /// Reports whether a remote-creation callback is installed.
    #[must_use]
    pub fn has_remote_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized function slot.
        unsafe { addr_of!((*self.as_ptr()).remote_cb).read() }.is_some()
    }

    /// Field: git_clone_options.remote_cb_payload
    /// Reports whether remote-callback state is installed.
    #[must_use]
    pub fn has_remote_callback_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { addr_of!((*self.as_ptr()).remote_cb_payload).read() }.is_null()
    }
}

impl<'object, 'data> GitCloneOptionsMut<'object, 'data> {
    /// Exclusively borrows a raw C options pointer, returning `None` for null.
    ///
    /// # Safety
    /// The shared-handle requirements apply, and no other access path may use
    /// the options for `'object`.
    pub unsafe fn from_ptr(ptr: *mut ffi::git_clone_options) -> Option<Self> {
        // SAFETY: the caller supplies a live exclusively accessible object.
        unsafe { GitCloneOptionsRef::from_ptr(ptr) }.map(Self)
    }

    /// Returns the writable C pointer for FFI calls and raw-place writes.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut ffi::git_clone_options {
        self.0.0.as_non_null().as_ptr().cast()
    }

    /// Reborrows this exclusive handle as shared.
    #[must_use]
    pub fn as_ref(&self) -> GitCloneOptionsRef<'_, 'data> {
        GitCloneOptionsRef(self.0.0)
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Selects whether clone creates a bare repository.
    pub fn set_bare(&mut self, bare: bool) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).bare).write(c_int::from(bare)) }
    }

    /// Replaces the local-clone policy.
    pub fn set_local(&mut self, local: GitCloneLocal) {
        // SAFETY: this exclusive handle permits the write, and the Rust enum
        // contains only published C values.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).local).write(local.into()) }
    }

    /// Stores an optional borrowed branch name.
    pub fn set_checkout_branch(&mut self, branch: Option<&'data CStr>) {
        let branch = branch.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write, and the
        // wrapper lifetime keeps a non-null string live and immutable.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_branch).write(branch) }
    }

    /// Exclusively borrows the embedded checkout options.
    #[must_use]
    pub fn checkout_options_mut(&mut self) -> GitCheckoutOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime and retains the outer data lifetime.
        unsafe { GitCheckoutOptionsMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Exclusively borrows the embedded fetch options.
    #[must_use]
    pub fn fetch_options_mut(&mut self) -> GitFetchOptionsMut<'_, 'data> {
        // SAFETY: raw-place projection through this exclusive handle reaches
        // the initialized inline field without forming a reference.
        let field = unsafe { addr_of_mut!((*self.as_mut_ptr()).fetch_opts) };
        // SAFETY: the projected field is exclusively borrowed for the returned
        // handle's lifetime and retains the outer data lifetime.
        unsafe { GitFetchOptionsMut::from_ptr(field) }.expect("an inline field is non-null")
    }

    /// Copies checkout options into the inline field.
    ///
    /// # Safety
    /// Callback invocations through the source, this copy, and any further C
    /// copies must not overlap. All nested callback state remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_checkout_options(&mut self, options: GitCheckoutOptionsRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).checkout_opts).write(options) }
    }

    /// Copies fetch options into the inline field.
    ///
    /// # Safety
    /// Callback invocations through the source, this copy, and any further C
    /// copies must not overlap. All nested callback state remains exclusively
    /// reserved for `'data`.
    pub unsafe fn set_fetch_options(&mut self, options: GitFetchOptionsRef<'_, 'data>) {
        // SAFETY: the source identifies an initialized borrowed header.
        let options = unsafe { options.as_ptr().read() };
        // SAFETY: this exclusive handle permits replacing the inline field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).fetch_opts).write(options) }
    }

    /// Installs a typed repository-creation callback and its payload.
    pub fn set_repository_callback<C: GitRepositoryCreateCallback>(
        &mut self,
        callback: &'data mut C,
    ) {
        let options = self.as_mut_ptr();
        // SAFETY: the function and payload slots are installed coherently, and
        // the wrapper lifetime reserves `callback` for every invocation.
        unsafe {
            addr_of_mut!((*options).repository_cb).write(Some(repository_create_trampoline::<C>));
            addr_of_mut!((*options).repository_cb_payload)
                .write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the repository-creation callback and payload.
    pub fn clear_repository_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits coherent writes to both slots.
        unsafe {
            addr_of_mut!((*options).repository_cb).write(None);
            addr_of_mut!((*options).repository_cb_payload).write(core::ptr::null_mut());
        }
    }

    /// Installs a typed remote-creation callback and its payload.
    pub fn set_remote_callback<C: GitRemoteCreateCallback>(&mut self, callback: &'data mut C) {
        let options = self.as_mut_ptr();
        // SAFETY: the function and payload slots are installed coherently, and
        // the wrapper lifetime reserves `callback` for every invocation.
        unsafe {
            addr_of_mut!((*options).remote_cb).write(Some(remote_create_trampoline::<C>));
            addr_of_mut!((*options).remote_cb_payload).write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the remote-creation callback and payload.
    pub fn clear_remote_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits coherent writes to both slots.
        unsafe {
            addr_of_mut!((*options).remote_cb).write(None);
            addr_of_mut!((*options).remote_cb_payload).write(core::ptr::null_mut());
        }
    }
}

unsafe extern "C" fn repository_create_trampoline<C: GitRepositoryCreateCallback>(
    out: *mut *mut ffi::git_repository,
    path: *const core::ffi::c_char,
    bare: c_int,
    payload: *mut c_void,
) -> c_int {
    let Some(out) = NonNull::new(out) else {
        return ffi::git_error_code_GIT_ERROR;
    };
    let Some(payload) = NonNull::new(payload.cast::<C>()) else {
        return ffi::git_error_code_GIT_ERROR;
    };
    if path.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: the installed trampoline pairs this monomorphization with a live,
    // exclusively reserved `C`, and libgit2 supplies a valid C string.
    let result = unsafe {
        payload
            .as_ptr()
            .as_mut()
            .unwrap()
            .create(CStr::from_ptr(path), bare != 0)
    };
    match result {
        Ok(repository) => {
            // SAFETY: `out` is the callback's writable result slot.
            unsafe { out.as_ptr().write(repository.into_raw()) };
            0
        }
        Err(error) => error,
    }
}

unsafe extern "C" fn remote_create_trampoline<C: GitRemoteCreateCallback>(
    out: *mut *mut ffi::git_remote,
    repository: *mut ffi::git_repository,
    name: *const core::ffi::c_char,
    url: *const core::ffi::c_char,
    payload: *mut c_void,
) -> c_int {
    let (Some(out), Some(payload)) = (NonNull::new(out), NonNull::new(payload.cast::<C>())) else {
        return ffi::git_error_code_GIT_ERROR;
    };
    if name.is_null() || url.is_null() {
        return ffi::git_error_code_GIT_ERROR;
    }
    // SAFETY: libgit2 supplies exclusive access to the repository for this
    // callback invocation; a null repository is rejected.
    let Some(mut repository) = (unsafe { GitRepositoryMut::from_ptr(repository) }) else {
        return ffi::git_error_code_GIT_ERROR;
    };
    // SAFETY: the installed trampoline pairs this monomorphization with a live,
    // exclusively reserved `C`; libgit2 supplies valid C strings.
    let result = unsafe {
        payload.as_ptr().as_mut().unwrap().create(
            &mut repository,
            CStr::from_ptr(name),
            CStr::from_ptr(url),
        )
    };
    match result {
        Ok(remote) => {
            // SAFETY: `out` is the callback's writable result slot, and the
            // clone operation keeps `repository` live until it frees the remote.
            unsafe { out.as_ptr().write(remote.into_raw()) };
            0
        }
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use super::*;

    #[test]
    fn clone_options_preserve_layout_and_defaults() {
        assert_eq!(
            size_of::<GitCloneOptions<'static>>(),
            size_of::<ffi::git_clone_options>()
        );
        assert_eq!(
            align_of::<GitCloneOptions<'static>>(),
            align_of::<ffi::git_clone_options>()
        );
        let options = GitCloneOptions::new();
        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_CLONE_OPTIONS_VERSION);
        assert_eq!(view.local(), Ok(GitCloneLocal::Auto));
        assert!(!view.bare());
        assert_eq!(view.checkout_branch(), None);
        assert_eq!(
            view.checkout_options().version(),
            ffi::GIT_CHECKOUT_OPTIONS_VERSION
        );
        assert_eq!(
            view.fetch_options().version(),
            ffi::GIT_FETCH_OPTIONS_VERSION as c_int
        );
        assert!(!view.has_repository_callback());
        assert!(!view.has_remote_callback());
    }

    #[test]
    fn setters_update_scalar_and_nested_fields() {
        let mut options = GitCloneOptions::new();
        {
            let mut view = options.as_mut();
            view.set_bare(true);
            view.set_local(GitCloneLocal::NoLinks);
            view.set_checkout_branch(Some(c"topic"));
            view.checkout_options_mut().set_disable_filters(true);
            view.fetch_options_mut()
                .set_depth(crate::api::remote::GitFetchDepth::new(3).unwrap());
        }
        let view = options.as_ref();
        assert!(view.bare());
        assert_eq!(view.local(), Ok(GitCloneLocal::NoLinks));
        assert_eq!(view.checkout_branch(), Some(c"topic"));
        assert!(view.checkout_options().disable_filters());
        assert_eq!(
            view.fetch_options().depth(),
            Ok(crate::api::remote::GitFetchDepth::new(3).unwrap())
        );
    }

    #[test]
    fn typed_callbacks_are_installed_and_invoked() {
        let mut repository_calls = 0;
        let mut repository_callback = |path: &CStr, bare: bool| {
            repository_calls += usize::from(path == c"repo" && bare);
            Err(17)
        };
        let repository_storage = Box::new(MaybeUninit::<ffi::git_repository>::zeroed());
        let repository = Box::into_raw(repository_storage).cast::<ffi::git_repository>();
        struct RejectRemote {
            expected_repository: *mut ffi::git_repository,
            calls: usize,
        }
        impl GitRemoteCreateCallback for RejectRemote {
            fn create<'repo>(
                &mut self,
                repository: &'repo mut GitRepositoryMut<'_>,
                name: &CStr,
                url: &CStr,
            ) -> Result<GitRemoteWithRepository<'repo>, i32> {
                self.calls += usize::from(
                    repository.as_ref().as_ptr() == self.expected_repository.cast_const()
                        && name == c"origin"
                        && url == c"url",
                );
                Err(23)
            }
        }
        let mut remote_callback = RejectRemote {
            expected_repository: repository,
            calls: 0,
        };
        let mut options = GitCloneOptions::new();
        {
            let mut view = options.as_mut();
            view.set_repository_callback(&mut repository_callback);
            view.set_remote_callback(&mut remote_callback);
        }
        let raw = options.as_ref().as_ptr();
        // SAFETY: the installed callback slots and payloads are read from the
        // live options value and invoked sequentially with valid arguments.
        unsafe {
            let repository_fn = addr_of!((*raw).repository_cb).read().unwrap();
            let repository_payload = addr_of!((*raw).repository_cb_payload).read();
            let mut repository_out = core::ptr::null_mut();
            assert_eq!(
                repository_fn(&mut repository_out, c"repo".as_ptr(), 1, repository_payload),
                17
            );

            let remote_fn = addr_of!((*raw).remote_cb).read().unwrap();
            let remote_payload = addr_of!((*raw).remote_cb_payload).read();
            let mut remote_out = core::ptr::null_mut();
            assert_eq!(
                remote_fn(
                    &mut remote_out,
                    repository,
                    c"origin".as_ptr(),
                    c"url".as_ptr(),
                    remote_payload,
                ),
                23
            );
        }
        drop(options);
        assert_eq!(repository_calls, 1);
        assert_eq!(remote_callback.calls, 1);
        // SAFETY: callback handles have expired and this recovers the exact
        // allocation returned by `Box::into_raw`.
        drop(unsafe { Box::from_raw(repository.cast::<MaybeUninit<ffi::git_repository>>()) });
    }

    /// The pinned `'data` still accepts referents that merely outlive the
    /// options, so invariance does not force `'static` on callers.
    #[test]
    fn pinned_options_accept_scoped_referents() {
        let branch = std::ffi::CString::new("topic").unwrap();
        let url = std::ffi::CString::new("http://scoped.invalid/").unwrap();
        let mut options = GitCloneOptions::new();
        {
            let mut view = options.as_mut();
            view.set_checkout_branch(Some(&branch));
            view.fetch_options_mut()
                .proxy_options_mut()
                .set_url(Some(&url));
        }
        let view = options.as_ref();
        assert_eq!(view.checkout_branch(), Some(branch.as_c_str()));
        assert_eq!(
            view.fetch_options().proxy_options().url(),
            Some(url.as_c_str())
        );
    }
}
