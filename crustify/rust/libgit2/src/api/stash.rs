//! Safe wrappers for libgit2 stash APIs.

use core::ffi::CStr;

use crate::oid::OidRef;
use crate::stash::StashApplyProgress;

/// Wraps: git_stash_apply_progress_cb
/// Safe callable surface for stash-application progress notifications.
pub trait GitStashApplyProgressCallback {
    /// Returns zero to continue, or a negative value to abort.
    fn call(&mut self, progress: StashApplyProgress) -> i32;
}

impl<F> GitStashApplyProgressCallback for F
where
    F: FnMut(StashApplyProgress) -> i32,
{
    fn call(&mut self, progress: StashApplyProgress) -> i32 {
        self(progress)
    }
}

/// Wraps: git_stash_cb
/// Safe callable surface for one transient stash-list entry.
pub trait GitStashCallback {
    /// Returns zero to continue iteration or nonzero to stop.
    fn call(&mut self, index: usize, message: &CStr, stash_id: OidRef<'_>) -> i32;
}

impl<F> GitStashCallback for F
where
    F: FnMut(usize, &CStr, OidRef<'_>) -> i32,
{
    fn call(&mut self, index: usize, message: &CStr, stash_id: OidRef<'_>) -> i32 {
        self(index, message, stash_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    #[test]
    fn callback_surfaces_pass_checked_values() {
        let mut progress = |value| i32::from(value == StashApplyProgress::Done);
        assert_eq!(
            GitStashApplyProgressCallback::call(&mut progress, StashApplyProgress::Done,),
            1
        );

        let oid = Oid::zeroed();
        let raw = core::ptr::addr_of!(oid)
            .cast::<crate::ffi::git_oid>()
            .cast_mut();
        // SAFETY: the local layout-compatible OID remains live throughout the call.
        let oid = unsafe { OidRef::from_ptr(raw) }.unwrap();
        let mut entry = |index, message: &CStr, _: OidRef<'_>| {
            i32::from(index == 2 && message == c"stash message")
        };
        assert_eq!(
            GitStashCallback::call(&mut entry, 2, c"stash message", oid),
            1
        );
    }
}
