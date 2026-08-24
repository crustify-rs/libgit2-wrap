//! Safe wrappers for libgit2 tree APIs.

use core::ffi::CStr;

use crate::tree::GitTreeEntryRef;

/// Wraps: git_treebuilder_filter_cb
/// Safe callable surface for inspecting one builder-owned tree entry.
pub trait GitTreebuilderFilterCallback {
    /// Returns `true` to remove `entry` from the builder.
    fn remove(&mut self, entry: GitTreeEntryRef<'_>) -> bool;
}

impl<F> GitTreebuilderFilterCallback for F
where
    F: FnMut(GitTreeEntryRef<'_>) -> bool,
{
    fn remove(&mut self, entry: GitTreeEntryRef<'_>) -> bool {
        self(entry)
    }
}

/// Wraps: git_treewalk_cb
/// Safe callable surface for one transient tree-walk entry.
pub trait GitTreewalkCallback {
    /// Receives the entry's relative root and borrowed entry.
    ///
    /// A positive result skips the entry and a negative result stops the walk.
    fn call(&mut self, root: &CStr, entry: GitTreeEntryRef<'_>) -> i32;
}

impl<F> GitTreewalkCallback for F
where
    F: FnMut(&CStr, GitTreeEntryRef<'_>) -> i32,
{
    fn call(&mut self, root: &CStr, entry: GitTreeEntryRef<'_>) -> i32 {
        self(root, entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_both_tree_callback_surfaces() {
        fn accepts_filter<C: GitTreebuilderFilterCallback>(_callback: C) {}
        fn accepts_walk<C: GitTreewalkCallback>(_callback: C) {}
        accepts_filter(|_: GitTreeEntryRef<'_>| false);
        accepts_walk(|_: &CStr, _: GitTreeEntryRef<'_>| 0);
    }
}
