//! Safe wrappers for libgit2 commit_graph APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CDropped, define_ctype};

use crate::ffi;

define_ctype!(
    /// Wraps: git_commit_graph
    /// An opaque commit-graph cache owned by libgit2.
    ///
    /// The object owns its path and its optional lazily opened commit-graph
    /// file. Owned pointers use [`GitCommitGraphOwned`] and release both with
    /// `git_commit_graph_free`.
    GitCommitGraph,
    GitCommitGraphRef,
    GitCommitGraphMut,
    ffi::git_commit_graph
);

/// An owned libgit2 commit graph.
pub type GitCommitGraphOwned = CBox<GitCommitGraph>;

// SAFETY: `git_commit_graph_free` is the destructor for one fully constructed
// commit graph and accepts null, although `CBox` only supplies a live non-null
// allocation. It disposes the owned path and optional file before freeing the
// graph, and `CBox` invokes it exactly once for an adopted owner.
ffibox::impl_dropped!(
    GitCommitGraph,
    ffi::git_commit_graph,
    ffi::git_commit_graph_free
);

define_ctype!(
    /// Wraps: git_commit_graph_writer
    /// An opaque writer for commit-graph files.
    ///
    /// The public C API constructs complete writer allocations and releases
    /// them with `git_commit_graph_writer_free`. Its internal path buffer,
    /// packed-commit vector, and object-ID type remain hidden behind this
    /// layout-compatible handle.
    GitCommitGraphWriter,
    GitCommitGraphWriterRef,
    GitCommitGraphWriterMut,
    ffi::git_commit_graph_writer
);

/// An owned commit-graph writer allocation.
pub type GitCommitGraphWriterOwned = CBox<GitCommitGraphWriter>;

// SAFETY: `git_commit_graph_writer_free` is the public destructor for a
// complete writer allocation. It accepts null, although `CBox` supplies one
// live non-null allocation exactly once, and disposes every owned field before
// freeing the writer storage.
unsafe impl CDropped for GitCommitGraphWriter {
    unsafe fn c_drop(writer: NonNull<Self>) {
        // SAFETY: the trait contract supplies one complete owned writer and
        // this wrapper is transparent over `ffi::git_commit_graph_writer`.
        unsafe { ffi::git_commit_graph_writer_free(writer.as_ptr().cast()) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};
    use core::ptr;

    use ffibox::{CCell, CDropped};

    use super::*;

    #[test]
    fn opaque_commit_graph_preserves_the_c_seam_and_drop_contract() {
        fn assert_cell<T: CCell>() {}
        fn assert_dropped<T: CDropped>() {}

        assert_cell::<GitCommitGraph>();
        assert_dropped::<GitCommitGraph>();
        assert_eq!(
            size_of::<GitCommitGraph>(),
            size_of::<ffi::git_commit_graph>()
        );
        assert_eq!(
            align_of::<GitCommitGraph>(),
            align_of::<ffi::git_commit_graph>()
        );
        assert_eq!(
            size_of::<GitCommitGraphRef<'_>>(),
            size_of::<*const ffi::git_commit_graph>()
        );
        assert_eq!(
            size_of::<GitCommitGraphMut<'_>>(),
            size_of::<*mut ffi::git_commit_graph>()
        );
        assert_eq!(
            size_of::<GitCommitGraphOwned>(),
            size_of::<*mut ffi::git_commit_graph>()
        );
    }

    #[test]
    fn null_commit_graph_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitCommitGraphRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitCommitGraphMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitCommitGraphOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

    #[test]
    fn writer_wrapper_matches_the_opaque_c_type() {
        assert_eq!(
            size_of::<GitCommitGraphWriter>(),
            size_of::<ffi::git_commit_graph_writer>()
        );
        assert_eq!(
            align_of::<GitCommitGraphWriter>(),
            align_of::<ffi::git_commit_graph_writer>()
        );
    }

    #[test]
    fn writer_registers_its_public_destructor() {
        fn requires_drop<T: CDropped>() {}
        requires_drop::<GitCommitGraphWriter>();
    }

    #[test]
    fn null_commit_graph_writer_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting a writer allocation.
        unsafe {
            assert!(GitCommitGraphWriterRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitCommitGraphWriterMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitCommitGraphWriterOwned::from_raw(ptr::null_mut()).is_none());
        }
    }
}
