//! Safe wrappers for libgit2 pack objects APIs.

use core::ffi::{CStr, c_void};
use core::marker::PhantomData;
use core::ptr::addr_of_mut;

use ffibox::CBox;

use crate::api::buffer::GitBufMut;
use crate::api::pack::GitPackbuilderStage;
use crate::ffi;
use crate::oid::OidRef;
use crate::pack::{GitPackbuilderForeachCallback, GitPackbuilderProgressCallback};
use crate::repository::GitRepositoryRef;
use crate::revwalk::GitRevwalkMut;

ffibox::define_ctype!(
    /// Wraps: git_packbuilder
    /// Opaque packfile-builder state managed by libgit2.
    ///
    /// The builder retains a borrowed repository pointer and may retain
    /// callback state. Construction and callback-registration wrappers must
    /// therefore keep those dependencies alive while the builder can use them.
    GitPackbuilder,
    GitPackbuilderRef,
    GitPackbuilderMut,
    ffi::git_packbuilder
);

/// An owned libgit2 packfile builder.
pub type GitPackbuilderOwned = CBox<GitPackbuilder>;

/// The progress callback a [`RepositoryPackbuilder`] hands to libgit2.
///
/// Boxing it once more gives the trait object a thin, address-stable slot to
/// be pointed at, which is what libgit2's `void *` payload requires.
type ProgressPayload = Box<dyn GitPackbuilderProgressCallback + Send>;

/// A packbuilder tied to the repository pointer retained by libgit2.
///
/// `inner` is declared before `progress` so that `git_packbuilder_free` runs
/// while the payload libgit2 still references is alive.
pub struct RepositoryPackbuilder<'repo> {
    inner: GitPackbuilderOwned,
    progress: Option<Box<ProgressPayload>>,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl RepositoryPackbuilder<'_> {
    /// Borrows the builder.
    pub fn as_ref(&self) -> GitPackbuilderRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the builder exclusively.
    pub fn as_mut(&mut self) -> GitPackbuilderMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_packbuilder_free` is the public destructor for a fully
// initialized, libgit2-allocated `git_packbuilder`. It releases the allocation
// and every field owned by it, and accepts null although `CBox` supplies a live
// non-null allocation.
ffibox::impl_dropped!(
    GitPackbuilder,
    ffi::git_packbuilder,
    ffi::git_packbuilder_free
);

/// Wraps: git_packbuilder_foreach
/// Streams the completed pack through a synchronous callback.
pub fn git_packbuilder_foreach<C>(
    builder: &mut GitPackbuilderMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitPackbuilderForeachCallback,
{
    unsafe extern "C" fn trampoline<C: GitPackbuilderForeachCallback>(
        buffer: *mut c_void,
        size: usize,
        payload: *mut c_void,
    ) -> i32 {
        if buffer.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the outer wrapper passes a live exclusive callback pointer
        // that outlives this synchronous invocation.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: libgit2 supplies a readable run of `size` bytes that stays
        // live for the invocation. The borrow is shared because the chunk is
        // storage libgit2 retains -- a stack header, the packbuilder's own
        // delta-base OID, or a shared `git_odb_object` cache entry -- and
        // hashes again after this callback returns; an exclusive slice would
        // assert a uniqueness the C side does not grant.
        let bytes = unsafe { core::slice::from_raw_parts(buffer.cast::<u8>(), size) };
        callback.call(bytes)
    }

    // SAFETY: callback and builder stay live for this synchronous call; the
    // trampoline reconstructs the exact callback type from its payload.
    let status = unsafe {
        ffi::git_packbuilder_foreach(
            builder.as_mut_ptr(),
            Some(trampoline::<C>),
            (callback as *mut C).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_free
/// Consumes an independently owned packbuilder.
pub fn git_packbuilder_free(builder: GitPackbuilderOwned) {
    drop(builder);
}

/// Wraps: git_packbuilder_hash
/// Borrows the inline pack hash the builder records when it writes a pack.
///
/// The result is never absent: C returns `&pb->pack_oid`, the address of an
/// inline field, so the borrow is always valid for the builder. Its digest is
/// all-zero until `git_packbuilder_write` has copied the indexer's hash into
/// it, which is what distinguishes "not written yet" here -- not a null
/// pointer, and not a distinct return value.
///
/// The C declaration takes a non-const builder, but the body performs no
/// write, so a shared borrow states the real contract.
#[must_use]
pub fn git_packbuilder_hash<'a>(builder: GitPackbuilderRef<'a>) -> OidRef<'a> {
    // SAFETY: this getter only reads the live builder; restoring mutability at
    // the seam merely satisfies the C declaration.
    let raw = unsafe { ffi::git_packbuilder_hash(builder.as_ptr().cast_mut()) };
    // SAFETY: the result is the address of an inline field of `builder`, so it
    // is non-null and live for `'a`.
    unsafe { OidRef::from_ptr(raw.cast_mut()) }.expect("an inline OID is non-null")
}

/// Wraps: git_packbuilder_insert
/// Inserts one object, with an optional display name.
pub fn git_packbuilder_insert(
    builder: &mut GitPackbuilderMut<'_>,
    oid: OidRef<'_>,
    name: Option<&CStr>,
) -> Result<(), i32> {
    // SAFETY: all arguments are live for the call and none is retained.
    let status = unsafe {
        ffi::git_packbuilder_insert(
            builder.as_mut_ptr(),
            oid.as_ptr(),
            name.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_insert_commit
/// Inserts a commit and its referenced tree.
pub fn git_packbuilder_insert_commit(
    builder: &mut GitPackbuilderMut<'_>,
    oid: OidRef<'_>,
) -> Result<(), i32> {
    // SAFETY: both handles are live and libgit2 retains neither pointer.
    let status = unsafe { ffi::git_packbuilder_insert_commit(builder.as_mut_ptr(), oid.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_insert_recur
/// Recursively inserts an object and everything it references.
pub fn git_packbuilder_insert_recur(
    builder: &mut GitPackbuilderMut<'_>,
    oid: OidRef<'_>,
    name: Option<&CStr>,
) -> Result<(), i32> {
    // SAFETY: all arguments are live for this synchronous call.
    let status = unsafe {
        ffi::git_packbuilder_insert_recur(
            builder.as_mut_ptr(),
            oid.as_ptr(),
            name.map_or(core::ptr::null(), CStr::as_ptr),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_insert_tree
/// Inserts a tree and all of its entries.
pub fn git_packbuilder_insert_tree(
    builder: &mut GitPackbuilderMut<'_>,
    oid: OidRef<'_>,
) -> Result<(), i32> {
    // SAFETY: both handles are live and libgit2 retains neither pointer.
    let status = unsafe { ffi::git_packbuilder_insert_tree(builder.as_mut_ptr(), oid.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_insert_walk
/// Consumes the remaining revisions from `walk` into the pack.
pub fn git_packbuilder_insert_walk(
    builder: &mut GitPackbuilderMut<'_>,
    walk: &mut GitRevwalkMut<'_>,
) -> Result<(), i32> {
    // SAFETY: both objects are exclusively borrowed for the stateful call.
    let status =
        unsafe { ffi::git_packbuilder_insert_walk(builder.as_mut_ptr(), walk.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_name
/// Borrows the completed pack's NUL-terminated name.
///
/// Unlike [`git_packbuilder_hash`], this really is optional: `pack_name` is
/// null in a freshly calloc'd builder and is only strdup'd once
/// `git_packbuilder_write` has committed the pack.
#[must_use]
pub fn git_packbuilder_name<'a>(builder: GitPackbuilderRef<'a>) -> Option<&'a CStr> {
    // SAFETY: this getter only reads stable storage inside the live builder.
    let raw = unsafe { ffi::git_packbuilder_name(builder.as_ptr().cast_mut()) };
    if raw.is_null() {
        None
    } else {
        // SAFETY: non-null results are NUL terminated and tied to `builder`.
        Some(unsafe { CStr::from_ptr(raw) })
    }
}

/// Wraps: git_packbuilder_new
/// Creates a packbuilder tied to the repository it retains.
pub fn git_packbuilder_new<'repo>(
    repo: GitRepositoryRef<'repo>,
) -> Result<RepositoryPackbuilder<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: `raw` is writable and `repo` remains alive through the returned
    // wrapper's lifetime.
    let status = unsafe { ffi::git_packbuilder_new(addr_of_mut!(raw), repo.as_ptr().cast_mut()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one fully initialized owned builder.
    let inner =
        unsafe { GitPackbuilderOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryPackbuilder {
        inner,
        progress: None,
        _repository: PhantomData,
    })
}

/// Wraps: git_packbuilder_object_count
/// Returns the number of objects scheduled for the pack.
pub fn git_packbuilder_object_count(builder: GitPackbuilderRef<'_>) -> usize {
    // SAFETY: this scalar getter does not mutate the live builder.
    unsafe { ffi::git_packbuilder_object_count(builder.as_ptr().cast_mut()) }
}

/// Derives the payload pointer libgit2 records from the callback box the
/// wrapper owns, cast to `void *` only at the FFI call.
///
/// The pointer is taken from an exclusive borrow, because
/// [`progress_trampoline`] writes through it, and the caller takes it only
/// once the box has reached its final home in the wrapper, because moving the
/// box afterwards would invalidate the pointer libgit2 kept.
fn progress_payload(stored: Option<&mut Box<ProgressPayload>>) -> *mut ProgressPayload {
    stored.map_or(core::ptr::null_mut(), |callback| {
        core::ptr::from_mut::<ProgressPayload>(callback.as_mut())
    })
}

/// The C entry point registered with libgit2 for progress reporting.
unsafe extern "C" fn progress_trampoline(
    stage: i32,
    current: u32,
    total: u32,
    payload: *mut c_void,
) -> i32 {
    if payload.is_null() {
        return -1;
    }
    let Ok(stage) = u32::try_from(stage) else {
        return -1;
    };
    let Ok(stage) = GitPackbuilderStage::try_from(stage) else {
        return -1;
    };
    // SAFETY: `payload` is the pointer `progress_payload` derived from the
    // boxed callback its wrapper still owns, so it addresses one initialized
    // `ProgressPayload` and carries provenance for writing to it. Libgit2
    // reports progress under the packbuilder's progress mutex, so at most one
    // such exclusive borrow exists at a time.
    let callback = unsafe { &mut *payload.cast::<ProgressPayload>() };
    callback.call(stage, current, total)
}

/// Wraps: git_packbuilder_set_callbacks
/// Installs or clears a thread-safe, `'static` progress callback.
pub fn git_packbuilder_set_callbacks(
    builder: &mut RepositoryPackbuilder<'_>,
    callback: Option<Box<dyn GitPackbuilderProgressCallback + Send>>,
) -> Result<(), i32> {
    // Install the replacement first, so the registered payload is derived
    // from the box that stays put, and keep the previous callback boxed until
    // libgit2 accepts the new one: a rejected call leaves libgit2 pointing at
    // the payload it already holds.
    let previous = core::mem::replace(&mut builder.progress, callback.map(Box::new));
    let payload = progress_payload(builder.progress.as_mut());
    let function: ffi::git_packbuilder_progress = if payload.is_null() {
        None
    } else {
        Some(progress_trampoline)
    };
    let mut handle = builder.inner.as_mut();
    // SAFETY: the handle exclusively borrows the live builder, and `payload`
    // is null or addresses the callback box this wrapper owns and keeps at a
    // stable address until a later successful replacement or its own drop.
    let status = unsafe {
        ffi::git_packbuilder_set_callbacks(handle.as_mut_ptr(), function, payload.cast())
    };
    if status == 0 {
        drop(previous);
        Ok(())
    } else {
        // Libgit2 refused the call without touching its registration, so the
        // payload it still points at goes back into the wrapper unchanged.
        builder.progress = previous;
        Err(status)
    }
}

/// Wraps: git_packbuilder_set_threads
/// Sets the worker count and returns the effective value.
pub fn git_packbuilder_set_threads(builder: &mut GitPackbuilderMut<'_>, threads: u32) -> u32 {
    // SAFETY: the builder is exclusively borrowed for this mutation.
    unsafe { ffi::git_packbuilder_set_threads(builder.as_mut_ptr(), threads) }
}

/// Wraps: git_packbuilder_write_buf
/// Writes the completed pack into `buffer`.
pub fn git_packbuilder_write_buf(
    buffer: &mut GitBufMut<'_>,
    builder: &mut GitPackbuilderMut<'_>,
) -> Result<(), i32> {
    // SAFETY: both independent objects are exclusively borrowed and live.
    let status =
        unsafe { ffi::git_packbuilder_write_buf(buffer.as_mut_ptr(), builder.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_packbuilder_written
/// Returns the number of objects already written.
pub fn git_packbuilder_written(builder: GitPackbuilderRef<'_>) -> usize {
    // SAFETY: this scalar getter does not mutate the live builder.
    unsafe { ffi::git_packbuilder_written(builder.as_ptr().cast_mut()) }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_representation_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitPackbuilder>();
        assert_dropped::<GitPackbuilder>();
        assert_eq!(
            size_of::<GitPackbuilder>(),
            size_of::<ffi::git_packbuilder>()
        );
        assert_eq!(
            align_of::<GitPackbuilder>(),
            align_of::<ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<GitPackbuilderRef<'_>>(),
            size_of::<*const ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<GitPackbuilderMut<'_>>(),
            size_of::<*mut ffi::git_packbuilder>()
        );
        assert_eq!(
            size_of::<Option<GitPackbuilderOwned>>(),
            size_of::<*mut ffi::git_packbuilder>()
        );
    }

    #[test]
    fn borrowed_handles_preserve_the_packbuilder_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_packbuilder>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_packbuilder>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage,
            // and the shared handle remains within this scope.
            let shared = unsafe { GitPackbuilderRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { GitPackbuilderMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from this `Box::into_raw`, no handle remains, and
        // the cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_packbuilder>>()) });
    }

    #[test]
    fn progress_payload_dispatches_through_the_owned_callback() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicI32, Ordering};

        let observed = Arc::new(AtomicI32::new(0));
        let sink = Arc::clone(&observed);
        let callback: ProgressPayload = Box::new(
            move |stage: GitPackbuilderStage, current: u32, total: u32| {
                sink.store(
                    ffi::git_packbuilder_stage_t::from(stage) as i32
                        + current as i32
                        + total as i32,
                    Ordering::SeqCst,
                );
                0
            },
        );
        let mut stored = Some(Box::new(callback));

        let payload = progress_payload(stored.as_mut());
        assert!(!payload.is_null());

        // SAFETY: `payload` was derived from `stored`, which is live and not
        // borrowed elsewhere for this call — the contract the wrapper upholds
        // for libgit2 by owning the box for as long as the registration lasts.
        let status = unsafe { progress_trampoline(1, 2, 3, payload.cast()) };
        assert_eq!(status, 0);
        assert_eq!(observed.load(Ordering::SeqCst), 6);

        // An invalid C stage is rejected without dispatching the callback.
        // SAFETY: the payload is still valid as above; the stage is an
        // ordinary C `int` whose value the trampoline must validate.
        let status = unsafe { progress_trampoline(2, 5, 6, payload.cast()) };
        assert_eq!(status, -1);
        assert_eq!(observed.load(Ordering::SeqCst), 6);

        drop(stored);
    }

    /// Holds one libgit2 initialization count for the duration of a test.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and
            // refcounted; this guard balances the successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances the successful initialization represented by
            // this guard, after every libgit2 owner has been dropped.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// The smallest on-disk bare repository `git_repository_open_bare`
    /// accepts, so a packbuilder can be built without a `git_repository_init`
    /// binding this crate does not need otherwise.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("crustify-packbuilder-{}-{tag}", std::process::id()));
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
    fn a_fresh_builder_reports_an_inline_hash_but_no_pack_name() {
        let _libgit2 = Libgit2Init::acquire();
        let repo_dir = BareRepo::create("fresh");
        let repo = crate::repository::git_repository_open_bare(&repo_dir.c_path())
            .expect("the hand-built directory is a bare repository");
        let builder = git_packbuilder_new(repo.as_ref()).expect("an empty packbuilder");

        // `git_packbuilder_hash` returns the address of an inline field, so it
        // is present even before a pack exists -- the "not written yet" state
        // is the all-zero digest, not an absent borrow.
        let hash = git_packbuilder_hash(builder.as_ref());
        assert!(crate::oid::git_oid_is_zero(hash));

        // `git_packbuilder_name`, by contrast, really is absent until
        // `git_packbuilder_write` strdups it.
        assert_eq!(git_packbuilder_name(builder.as_ref()), None);

        assert_eq!(git_packbuilder_object_count(builder.as_ref()), 0);
        assert_eq!(git_packbuilder_written(builder.as_ref()), 0);
    }

    #[test]
    fn a_cleared_progress_callback_registers_no_payload() {
        assert!(progress_payload(None).is_null());

        // SAFETY: a null payload is what a cleared registration installs; the
        // trampoline must reject it instead of dereferencing it.
        let status = unsafe { progress_trampoline(0, 0, 0, core::ptr::null_mut()) };
        assert_eq!(status, -1);
    }
}
