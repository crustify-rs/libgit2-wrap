//! Safe wrappers for libgit2 commit APIs.

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
        parents: &[crate::commit::GitCommitRef<'_>],
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
        &[crate::commit::GitCommitRef<'_>],
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
        parents: &[crate::commit::GitCommitRef<'_>],
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
             _: &[crate::commit::GitCommitRef<'_>]| 0,
        );
    }
}
