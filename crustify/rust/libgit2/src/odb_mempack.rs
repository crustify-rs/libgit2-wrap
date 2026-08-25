//! Safe wrappers for libgit2 odb_mempack APIs.

use core::ptr::{NonNull, addr_of};

use ffibox::CBox;

use crate::api::buffer::GitBufMut;
use crate::ffi;
use crate::repository::GitRepositoryMut;
use crate::sys::odb_backend::{GitOdbBackendMut, GitOdbBackendOwned, GitOdbBackendRef};

ffibox::define_ctype!(
    /// Wraps: git_odb_backend
    /// The in-memory packer's view of an object-database backend.
    ///
    /// This is the same allocation and the same layout as
    /// [`GitOdbBackend`](crate::sys::odb_backend::GitOdbBackend); what it adds
    /// is the proof that the allocation really is a mempack.
    /// `git_mempack_dump` and `git_mempack_reset` cast their
    /// `git_odb_backend *` straight to the private `memory_packer_db` that
    /// `git_mempack_new` allocates and then read its object map and commit
    /// array through that cast. Neither inspects the callback table first, so
    /// handing them any other concrete backend is type confusion rather than
    /// an error return, and the polymorphic backend handle — which
    /// `git_odb_stream`, `git_odb_writepack` and the object database hand out
    /// for backends of every kind — cannot carry that guarantee.
    ///
    /// [`git_mempack_new`] is therefore the only safe source of one of these
    /// handles; recovering it after the backend has been given to an object
    /// database goes through the explicitly unsafe
    /// [`GitMempackBackendRef::from_odb_backend`].
    GitMempackBackend,
    GitMempackBackendRef,
    GitMempackBackendMut,
    ffi::git_odb_backend
);

/// An exclusively owned mempack backend that has not been installed in an
/// object database.
pub type GitMempackBackendOwned = CBox<GitMempackBackend>;

/// Field: git_odb_backend.free
// SAFETY: a `GitMempackBackend` is only ever adopted from `git_mempack_new`,
// which installs `impl__free` in the shared `git_odb_backend` header. `CBox`
// supplies one live, uniquely owned allocation and invokes this exactly once.
unsafe impl ffibox::CDropped for GitMempackBackend {
    unsafe fn c_drop(backend: NonNull<Self>) {
        let backend = backend.as_ptr().cast::<ffi::git_odb_backend>();
        // SAFETY: raw-place projection reads the initialized callback slot of
        // the live backend supplied by the `CDropped` contract.
        let free = unsafe { addr_of!((*backend).free).read() }
            .expect("git_mempack_new installs a free callback");
        // SAFETY: this is the mempack's own destructor and the owner grants
        // its one final invocation.
        unsafe { free(backend) }
    }
}

impl GitMempackBackend {
    /// Surrenders the mempack view so the backend can be installed in an
    /// object database with
    /// [`git_odb_add_backend`](crate::odb::git_odb_add_backend).
    pub fn into_odb_backend(backend: GitMempackBackendOwned) -> GitOdbBackendOwned {
        // SAFETY: both wrappers are transparent over `git_odb_backend` and
        // dispatch teardown through the same installed `free` callback, so the
        // released owner transfers unchanged and is still dropped exactly once.
        unsafe { GitOdbBackendOwned::from_raw(backend.into_raw()) }
            .expect("an owned backend pointer is non-null")
    }
}

impl<'a> GitMempackBackendRef<'a> {
    /// Borrows this mempack as a polymorphic object-database backend.
    #[must_use]
    pub fn as_odb_backend(&self) -> GitOdbBackendRef<'a> {
        // SAFETY: the handles are transparent over the same live C object and
        // carry the same borrow; the shared view adds no access.
        unsafe { GitOdbBackendRef::from_ptr(self.as_ptr().cast_mut()) }
            .expect("a live handle is non-null")
    }

    /// Recovers the mempack view of a backend, typically after
    /// [`git_odb_add_backend`](crate::odb::git_odb_add_backend) has taken the
    /// owner and the database is keeping the allocation alive.
    ///
    /// # Safety
    ///
    /// `backend` must address an allocation produced by [`git_mempack_new`].
    /// The mempack entry points reinterpret it as their private
    /// `memory_packer_db`, so any other concrete backend is undefined
    /// behaviour rather than a rejected argument.
    #[must_use]
    pub unsafe fn from_odb_backend(backend: GitOdbBackendRef<'a>) -> Self {
        // SAFETY: the caller asserts the concrete type; liveness and the
        // borrow lifetime come from the handle being converted.
        unsafe { Self::from_ptr(backend.as_ptr().cast_mut()) }.expect("a live handle is non-null")
    }
}

impl<'a> GitMempackBackendMut<'a> {
    /// Borrows this mempack exclusively as a polymorphic backend.
    #[must_use]
    pub fn as_odb_backend_mut(&mut self) -> GitOdbBackendMut<'_> {
        // SAFETY: the handles are transparent over the same live C object and
        // the reborrow keeps the exclusive access single-pathed.
        unsafe { GitOdbBackendMut::from_ptr(self.as_mut_ptr()) }.expect("a live handle is non-null")
    }

    /// Recovers the exclusive mempack view of a backend.
    ///
    /// # Safety
    ///
    /// As [`GitMempackBackendRef::from_odb_backend`], and the exclusive access
    /// the converted handle carries must remain the only one.
    #[must_use]
    pub unsafe fn from_odb_backend_mut(mut backend: GitOdbBackendMut<'a>) -> Self {
        // SAFETY: the caller asserts the concrete type; liveness, exclusivity
        // and the borrow lifetime come from the handle being converted.
        unsafe { Self::from_ptr(backend.as_mut_ptr()) }.expect("a live handle is non-null")
    }
}

/// Wraps: git_mempack_dump
/// Writes the mempack's queued commits to a thin packfile buffer.
///
/// `pack` receives a freshly built packfile; the repository is borrowed
/// exclusively because building it runs a packbuilder against the repository's
/// object database and caches.
pub fn git_mempack_dump(
    pack: &mut GitBufMut<'_>,
    repository: &mut GitRepositoryMut<'_>,
    backend: GitMempackBackendRef<'_>,
) -> Result<(), i32> {
    // SAFETY: the output and repository are exclusively borrowed, while the
    // backend is a live mempack whose queued commits are only read. No pointer
    // is retained.
    let status = unsafe {
        ffi::git_mempack_dump(
            pack.as_mut_ptr(),
            repository.as_mut_ptr(),
            backend.as_ptr().cast_mut(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_mempack_new
/// Creates a new in-memory object-database backend.
pub fn git_mempack_new() -> Result<GitMempackBackendOwned, i32> {
    let mut out = core::ptr::null_mut();
    // SAFETY: `out` is a writable slot for the newly allocated backend.
    let status = unsafe { ffi::git_mempack_new(&mut out) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success transfers one fully constructed mempack backend with its
    // required destructor callback installed.
    unsafe { GitMempackBackendOwned::from_raw(out) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_mempack_reset
/// Frees all queued objects while retaining the backend for reuse.
pub fn git_mempack_reset(backend: &mut GitMempackBackendMut<'_>) -> Result<(), i32> {
    // SAFETY: the exclusive live mempack handle permits clearing its private
    // object map and commit array; C retains no pointer.
    let status = unsafe { ffi::git_mempack_reset(backend.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::buffer::GitBuf;

    /// The smallest on-disk bare repository `git_repository_open_bare`
    /// accepts, so a packbuilder can be built without a `git_repository_init`
    /// binding this crate does not need otherwise.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-mempack-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a private temporary directory");
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
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for BareRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn empty_mempack_can_be_reset_and_dropped() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after the backend and all of its allocations have been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut backend = git_mempack_new().unwrap();
        git_mempack_reset(&mut backend.as_mut()).unwrap();
        drop(backend);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn an_empty_mempack_dumps_a_well_formed_pack() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after every owner created here has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let directory = BareRepo::create("dump");
        let mut repository = crate::repository::git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");
        let backend = git_mempack_new().unwrap();

        let mut pack = GitBuf::new();
        git_mempack_dump(
            &mut pack.as_mut(),
            &mut repository.as_mut(),
            backend.as_ref(),
        )
        .expect("an empty mempack still writes a pack");

        // A pack with no objects is still a complete file: the "PACK" magic,
        // version 2, a zero object count and the trailing checksum.
        let written = pack.as_ref();
        let contents = written.contents().expect("a written pack");
        let header: Vec<u8> = contents.elems().take(12).collect();
        assert_eq!(&header[..4], b"PACK");
        assert_eq!(&header[4..8], &[0, 0, 0, 2]);
        assert_eq!(&header[8..12], &[0, 0, 0, 0]);
        assert_eq!(written.size(), 12 + 20);

        drop(pack);
        drop(backend);
        drop(repository);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }

    #[test]
    fn the_mempack_view_converts_to_and_from_the_polymorphic_backend() {
        use core::mem::{align_of, size_of};

        assert_eq!(
            size_of::<GitMempackBackend>(),
            size_of::<ffi::git_odb_backend>()
        );
        assert_eq!(
            align_of::<GitMempackBackend>(),
            align_of::<ffi::git_odb_backend>()
        );

        // SAFETY: process-global initialization is refcounted and balanced
        // once the single backend owner below has been dropped.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut mempack = git_mempack_new().unwrap();
        let address = mempack.as_ref().as_ptr();
        assert_eq!(mempack.as_ref().as_odb_backend().as_ptr(), address);
        assert_eq!(
            mempack.as_mut().as_odb_backend_mut().as_ref().as_ptr(),
            address
        );

        // Surrendering the view keeps the same allocation and the same single
        // owner, which still drops through the installed mempack destructor.
        let backend = GitMempackBackend::into_odb_backend(mempack);
        assert_eq!(backend.as_ref().as_ptr(), address);
        // SAFETY: this backend is the allocation `git_mempack_new` produced
        // immediately above, and no other handle to it is in use.
        let recovered = unsafe { GitMempackBackendRef::from_odb_backend(backend.as_ref()) };
        assert_eq!(recovered.as_ptr(), address);
        drop(backend);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
