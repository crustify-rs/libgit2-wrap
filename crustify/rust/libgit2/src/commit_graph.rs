//! Safe wrappers for libgit2 commit_graph APIs.

use ffibox::{CBox, define_ctype};

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
}
