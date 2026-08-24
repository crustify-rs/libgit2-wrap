//! Safe wrappers for libgit2 revwalk APIs.

use crate::oid::OidRef;

/// Wraps: git_revwalk_hide_cb
/// Safe callable surface for deciding whether to hide a transient commit ID.
pub trait GitRevwalkHideCallback {
    /// Returns nonzero to hide this commit and its ancestors.
    fn call(&mut self, commit_id: OidRef<'_>) -> i32;
}

impl<F> GitRevwalkHideCallback for F
where
    F: FnMut(OidRef<'_>) -> i32,
{
    fn call(&mut self, commit_id: OidRef<'_>) -> i32 {
        self(commit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    #[test]
    fn callback_receives_a_typed_oid_handle() {
        let oid = Oid::zeroed();
        let raw = core::ptr::addr_of!(oid)
            .cast::<crate::ffi::git_oid>()
            .cast_mut();
        // SAFETY: `raw` addresses the live layout-compatible local OID for the
        // duration of this callback invocation.
        let oid = unsafe { OidRef::from_ptr(raw) }.unwrap();
        let mut callback = |candidate: OidRef<'_>| i32::from(candidate.as_ptr() == oid.as_ptr());
        assert_eq!(GitRevwalkHideCallback::call(&mut callback, oid), 1);
    }
}
