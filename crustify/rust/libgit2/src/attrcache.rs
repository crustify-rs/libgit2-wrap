//! Safe wrappers for libgit2 attrcache APIs.

/// Wraps: git_attr_cache_flush
/// Clears the repository attribute cache.
pub fn git_attr_cache_flush(repo: &mut crate::repository::GitRepositoryMut<'_>) {
    // SAFETY: the exclusive repository handle prevents safe attribute-string
    // borrows from coexisting with this cache-invalidating operation.
    let status = unsafe { crate::ffi::git_attr_cache_flush(repo.as_mut_ptr()) };
    debug_assert_eq!(status, 0);
}
