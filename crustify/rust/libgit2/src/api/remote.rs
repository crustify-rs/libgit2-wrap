//! Safe wrappers for libgit2 remote APIs.

use core::ffi::CStr;

use crate::api::buffer::GitBufMut;
use crate::util::net::Direction;

/// Wraps: git_url_resolve_cb
/// Safe callable surface for the deprecated remote URL resolver.
pub trait GitUrlResolveCallback {
    /// Writes the resolved URL to `output`.
    ///
    /// Return zero on success, `GIT_PASSTHROUGH`, or another libgit2 error.
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32;
}

impl<F> GitUrlResolveCallback for F
where
    F: FnMut(&mut GitBufMut<'_>, &CStr, Direction) -> i32,
{
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32 {
        self(output, url, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_the_url_resolver_surface() {
        fn accepts<C: GitUrlResolveCallback>(_callback: C) {}
        accepts(|_: &mut GitBufMut<'_>, _: &CStr, _: Direction| 0);
    }
}
