//! Safe wrappers for libgit2 tag APIs.

use core::ffi::CStr;

use crate::oid::OidRef;

/// Wraps: git_tag_foreach_cb
/// Safe callable surface for one transient tag-list entry.
pub trait GitTagForeachCallback {
    /// Receives a tag reference name and its object ID.
    ///
    /// A nonzero return value stops iteration.
    fn call(&mut self, name: &CStr, oid: OidRef<'_>) -> i32;
}

impl<F> GitTagForeachCallback for F
where
    F: FnMut(&CStr, OidRef<'_>) -> i32,
{
    fn call(&mut self, name: &CStr, oid: OidRef<'_>) -> i32 {
        self(name, oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_the_tag_callback_surface() {
        fn accepts<C: GitTagForeachCallback>(_callback: C) {}
        accepts(|_: &CStr, _: OidRef<'_>| 0);
    }
}
