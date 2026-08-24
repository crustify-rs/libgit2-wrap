//! Safe wrappers for libgit2 submodule APIs.

use core::ffi::CStr;

use crate::submodule::GitSubmoduleMut;

/// Wraps: git_submodule_cb
/// Safe callable surface for one transient submodule visit.
pub trait GitSubmoduleCallback {
    /// Returns zero to continue iteration or a nonzero status to stop.
    fn call(&mut self, submodule: GitSubmoduleMut<'_>, name: &CStr) -> i32;
}

impl<F> GitSubmoduleCallback for F
where
    F: FnMut(GitSubmoduleMut<'_>, &CStr) -> i32,
{
    fn call(&mut self, submodule: GitSubmoduleMut<'_>, name: &CStr) -> i32 {
        self(submodule, name)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn callback_receives_an_exclusive_typed_handle() {
        let storage = Box::new(MaybeUninit::<crate::ffi::git_submodule>::zeroed());
        let raw = Box::into_raw(storage).cast::<crate::ffi::git_submodule>();
        // SAFETY: `raw` addresses live opaque storage and this scope has
        // exclusive access for the callback invocation.
        let submodule = unsafe { GitSubmoduleMut::from_ptr(raw) }.unwrap();
        let mut callback = |value: GitSubmoduleMut<'_>, name: &CStr| {
            i32::from(value.as_ref().as_ptr() == raw.cast_const() && name == c"child")
        };
        assert_eq!(
            GitSubmoduleCallback::call(&mut callback, submodule, c"child"),
            1
        );
        // SAFETY: the callback handle was consumed and dropped; this recovers
        // the exact allocation returned by `Box::into_raw`.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<crate::ffi::git_submodule>>()) });
    }
}
